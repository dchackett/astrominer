use crate::api::*;
use bevy::math::Vec2;

pub struct CodexAI;

impl CodexAI {
    pub fn new() -> Self {
        Self
    }

    fn steer_rocket(rocket: &RocketView, target_delta: Vec2) -> RocketCommand {
        let desired = target_delta.normalize_or_zero();
        let forward = rocket.forward();
        let cross = forward.x * desired.y - forward.y * desired.x;
        let dot = forward.dot(desired);

        let mut cmd = RocketCommand::default();
        // Positive command rotates clockwise; invert cross to steer toward target.
        cmd.rotation = (-cross * 2.2).clamp(-1.0, 1.0);
        cmd.thrust = if dot > 0.2 { 1.0 } else { 0.35 };
        cmd
    }
}

impl PlayerAI for CodexAI {
    fn name(&self) -> &str {
        "Codex"
    }

    fn tick(&mut self, state: &GameStateView) -> Commands {
        let mut cmds = Commands::default();

        let enemy_pressure = state.enemy_rockets.len() > state.my_rockets.len();
        if state.my_station.build_progress.is_none() && state.my_station.build_queue_length == 0 {
            let want_tug = state.my_tugs.len() < 3
                || (state.my_tugs.len() < 5
                    && state.my_rockets.len() > state.my_tugs.len() * 2
                    && state.my_station.resources >= 80.0);
            let want_rocket = state.my_rockets.len() < 5
                || enemy_pressure
                || (state.my_station.resources >= 120.0 && state.my_rockets.len() < 14);

            cmds.station.build = if want_tug {
                Some(UnitTypeView::Tug)
            } else if want_rocket {
                Some(UnitTypeView::Rocket)
            } else {
                None
            };
        }

        for rocket in &state.my_rockets {
            let rocket_pos = rocket.position;
            let target = state.enemy_rockets.iter().min_by(|a, b| {
                state
                    .distance(rocket_pos, a.position)
                    .partial_cmp(&state.distance(rocket_pos, b.position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let target_pos = target
                .map(|r| r.position)
                .unwrap_or(state.enemy_station.position);
            let delta = state.shortest_delta(rocket.position, target_pos);
            let delta_v = Vec2::new(delta[0], delta[1]);

            let mut cmd = Self::steer_rocket(rocket, delta_v);
            let dist = delta_v.length();
            let forward = rocket.forward();
            let facing = forward.dot(delta_v.normalize_or_zero());
            let can_shoot = rocket.shoot_cooldown <= 0.0;
            cmd.shoot = can_shoot && facing > 0.93 && dist < 2600.0;

            if dist < 360.0 {
                // Close-range collision avoidance.
                cmd.thrust = 0.0;
            }

            cmds.rockets.insert(rocket.id, cmd);
        }

        let mut assigned = std::collections::HashSet::new();
        for tug in &state.my_tugs {
            let mut cmd = TugCommand::default();
            let tug_pos = tug.position;

            if let Some(carry_id) = tug.carrying {
                let to_home = state.shortest_delta(tug.position, state.my_station.position);
                let home_vec = Vec2::new(to_home[0], to_home[1]).normalize_or_zero() * 90.0;
                cmd.thrust = [home_vec.x, home_vec.y];
                cmd.beam_target = Some(carry_id);
                cmds.tugs.insert(tug.id, cmd);
                continue;
            }

            let gatherable = state
                .asteroids
                .iter()
                .filter(|a| a.tier <= 2 && !assigned.contains(&a.id))
                .min_by(|a, b| {
                    state
                        .distance(tug_pos, a.position)
                        .partial_cmp(&state.distance(tug_pos, b.position))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some(asteroid) = gatherable {
                assigned.insert(asteroid.id);
                let to_rock = state.shortest_delta(tug.position, asteroid.position);
                let pull_vec = Vec2::new(to_rock[0], to_rock[1]).normalize_or_zero() * 90.0;
                cmd.thrust = [pull_vec.x, pull_vec.y];
                cmd.beam_target = Some(asteroid.id);
            } else {
                let to_mid = state.shortest_delta(tug.position, state.enemy_station.position);
                let roam = Vec2::new(to_mid[0], to_mid[1]).normalize_or_zero() * 50.0;
                cmd.thrust = [roam.x, roam.y];
            }

            cmds.tugs.insert(tug.id, cmd);
        }

        let mut beam_count = 0usize;
        for bullet in &state.bullets {
            if beam_count >= 5 || bullet.team == state.my_team {
                continue;
            }
            let delta = state.shortest_delta(state.my_station.position, bullet.position);
            let delta_vec = Vec2::new(delta[0], delta[1]);
            let dist = delta_vec.length();
            if dist > state.my_station.beam_radius {
                continue;
            }

            let away = delta_vec.normalize_or_zero();
            cmds.station.beam_targets.push(BeamCommand {
                target: bullet.id,
                force_direction: [away.x, away.y],
            });
            beam_count += 1;
        }

        let repair_candidate = state
            .my_rockets
            .iter()
            .map(|r| (r.id, r.health / r.max_health, r.position))
            .chain(
                state
                    .my_tugs
                    .iter()
                    .map(|t| (t.id, t.health / t.max_health, t.position)),
            )
            .filter(|(_, ratio, pos)| {
                *ratio < 0.9
                    && state.distance(state.my_station.position, *pos)
                        <= state.my_station.beam_radius
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((id, _, _)) = repair_candidate {
            cmds.station.repair_target = Some(id);
        }

        cmds
    }
}

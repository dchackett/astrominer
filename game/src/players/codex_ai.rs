use crate::api::*;
use bevy::math::Vec2;
use std::collections::HashMap;

pub struct CodexAI {
    tug_targets: HashMap<EntityId, EntityId>,
}

impl CodexAI {
    pub fn new() -> Self {
        Self {
            tug_targets: HashMap::new(),
        }
    }
}

impl PlayerAI for CodexAI {
    fn name(&self) -> &str {
        "Codex"
    }

    fn tick(&mut self, state: &GameStateView) -> Commands {
        let mut cmds = Commands::default();
        let beam_radius = state.my_station.beam_radius;

        if state.my_station.build_progress.is_none() && state.my_station.build_queue_length == 0 {
            let need_tugs = state.my_tugs.is_empty()
                || (state.tick < 1500
                    && state.my_tugs.len() < 2
                    && state.enemy_rockets.len() + 2 < state.my_rockets.len()
                    && state.my_station.resources > 75.0);
            let need_rockets = state.my_rockets.len() < 7
                || state.enemy_rockets.len() >= state.my_rockets.len()
                || (state.my_station.resources >= 120.0 && state.my_rockets.len() < 18);
            let build = if need_tugs {
                Some(UnitTypeView::Tug)
            } else if need_rockets {
                Some(UnitTypeView::Rocket)
            } else {
                None
            };
            cmds.station.build = build;
        }

        for rocket in &state.my_rockets {
            let nearest_threat = state.enemy_rockets.iter().min_by(|a, b| {
                state
                    .distance(rocket.position, a.position)
                    .partial_cmp(&state.distance(rocket.position, b.position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let station_threat = state
                .enemy_rockets
                .iter()
                .filter(|r| state.distance(r.position, state.my_station.position) < 3600.0)
                .min_by(|a, b| {
                    state
                        .distance(a.position, state.my_station.position)
                        .partial_cmp(&state.distance(b.position, state.my_station.position))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            let (target_pos, target_vel, standoff) = if let Some(t) = station_threat {
                (t.position, t.velocity, 180.0)
            } else if let Some(t) =
                nearest_threat.filter(|t| state.distance(rocket.position, t.position) < 2200.0)
            {
                (t.position, t.velocity, 220.0)
            } else {
                (state.enemy_station.position, [0.0, 0.0], 320.0)
            };

            let cmd = fly_and_shoot(state, rocket, target_pos, target_vel, standoff);
            cmds.rockets.insert(rocket.id, cmd);
        }

        self.tug_targets.retain(|tid, aid| {
            state.my_tugs.iter().any(|t| t.id == *tid)
                && state.asteroids.iter().any(|a| a.id == *aid && a.tier <= 2)
        });
        let mut claimed: Vec<EntityId> = state.my_tugs.iter().filter_map(|t| t.carrying).collect();
        claimed.extend(self.tug_targets.values().copied());

        for tug in &state.my_tugs {
            let mut cmd = TugCommand::default();
            let tug_vel = tug.velocity_vec2();
            if tug.carrying.is_some() {
                let to_station = dv2(state, tug.position, state.my_station.position);
                let dist = to_station.length();
                let dir = to_station.normalize_or_zero();
                if dist < beam_radius - 32.0 {
                    cmd.beam_target = None;
                    let escape = -dir * 90.0;
                    cmd.thrust = [escape.x, escape.y];
                    self.tug_targets.remove(&tug.id);
                } else {
                    let desired_speed = if dist < beam_radius + 80.0 {
                        40.0
                    } else {
                        160.0
                    };
                    let desired_v = dir * desired_speed;
                    let dv = desired_v - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = tug.carrying;
                }
                cmds.tugs.insert(tug.id, cmd);
                continue;
            }

            let target_id = self
                .tug_targets
                .get(&tug.id)
                .and_then(|id| {
                    state
                        .asteroids
                        .iter()
                        .find(|a| a.id == *id && a.tier <= 2)
                        .map(|a| a.id)
                })
                .unwrap_or_else(|| {
                    let best = state
                        .asteroids
                        .iter()
                        .filter(|a| a.tier <= 2 && !claimed.contains(&a.id))
                        .min_by(|a, b| {
                            let sa = state.distance(tug.position, a.position)
                                + state.distance(a.position, state.my_station.position) * 0.6;
                            let sb = state.distance(tug.position, b.position)
                                + state.distance(b.position, state.my_station.position) * 0.6;
                            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    if let Some(ast) = best {
                        self.tug_targets.insert(tug.id, ast.id);
                        claimed.push(ast.id);
                        ast.id
                    } else {
                        EntityId(0)
                    }
                });

            if let Some(target) = state.asteroids.iter().find(|a| a.id == target_id) {
                let to_target = dv2(state, tug.position, target.position);
                let dist = to_target.length();
                let desired_v = to_target.normalize_or_zero() * 155.0;
                let dv = desired_v - tug_vel;
                cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                if dist < 120.0 {
                    cmd.beam_target = Some(target.id);
                }
            } else {
                let away =
                    -dv2(state, tug.position, state.my_station.position).normalize_or_zero() * 80.0;
                cmd.thrust = [away.x, away.y];
            }

            cmds.tugs.insert(tug.id, cmd);
        }

        for bullet in &state.bullets {
            if cmds.station.beam_targets.len() >= 5 || bullet.team == state.my_team {
                continue;
            }
            let to_bullet = dv2(state, state.my_station.position, bullet.position);
            if to_bullet.length() > beam_radius {
                continue;
            }
            let toward_station = -to_bullet.normalize_or_zero();
            let bv = bullet.velocity_vec2();
            if bv.dot(toward_station) < 40.0 {
                continue;
            }
            let perp = Vec2::new(-bv.y, bv.x).normalize_or_zero();
            cmds.station.beam_targets.push(BeamCommand {
                target: bullet.id,
                force_direction: [perp.x, perp.y],
            });
        }

        if cmds.station.beam_targets.len() < 5 {
            for ast in state.asteroids.iter().filter(|a| {
                a.tier <= 2 && state.distance(a.position, state.my_station.position) <= beam_radius
            }) {
                if cmds.station.beam_targets.len() >= 5 {
                    break;
                }
                let pull = dv2(state, ast.position, state.my_station.position).normalize_or_zero();
                cmds.station.beam_targets.push(BeamCommand {
                    target: ast.id,
                    force_direction: [pull.x, pull.y],
                });
            }
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
                *ratio < 0.9 && state.distance(state.my_station.position, *pos) <= beam_radius
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        if let Some((id, _, _)) = repair_candidate {
            cmds.station.repair_target = Some(id);
        }

        cmds
    }
}

fn dv2(state: &GameStateView, from: [f32; 2], to: [f32; 2]) -> Vec2 {
    let d = state.shortest_delta(from, to);
    Vec2::new(d[0], d[1])
}

fn fly_and_shoot(
    state: &GameStateView,
    rocket: &RocketView,
    target_pos: [f32; 2],
    target_vel: [f32; 2],
    standoff: f32,
) -> RocketCommand {
    let mut cmd = RocketCommand::default();
    let rocket_vel = rocket.velocity_vec2();
    let forward = rocket.forward();
    let to_target = dv2(state, rocket.position, target_pos);
    let dist = to_target.length();
    let target_v = Vec2::new(target_vel[0], target_vel[1]);

    let bullet_speed = 500.0;
    let closing = -(target_v - rocket_vel).dot(to_target.normalize_or_zero());
    let effective = (bullet_speed + closing.max(0.0)).max(60.0);
    let t_hit = if dist > 5.0 { dist / effective } else { 0.0 };
    let aim = (to_target + target_v * t_hit).normalize_or_zero();

    let cross = forward.perp_dot(aim);
    cmd.rotation = cross.clamp(-1.0, 1.0);

    let desired_speed = if dist > standoff + 220.0 {
        210.0
    } else if dist < standoff - 80.0 {
        -80.0
    } else {
        45.0
    };
    let mut desired_v = to_target.normalize_or_zero() * desired_speed + target_v * 0.3;
    if dist < 700.0 {
        let perp = Vec2::new(-to_target.y, to_target.x).normalize_or_zero();
        let strafe_sign = if rocket.id.0 % 2 == 0 { 1.0 } else { -1.0 };
        desired_v += perp * strafe_sign * 45.0;
    }
    desired_v += bullet_dodge(state, rocket) * 0.8;
    let dv = desired_v - rocket_vel;
    let dv_len = dv.length();
    if dv_len > 5.0 {
        let burn_dir = dv / dv_len;
        let align = forward.dot(burn_dir);
        cmd.thrust = if align > 0.25 {
            (dv_len / 35.0).clamp(0.2, 1.0)
        } else if align > -0.2 {
            0.1
        } else {
            0.0
        };
    }

    if rocket.shoot_cooldown <= 0.0
        && dist < 730.0
        && forward.dot(aim) > 0.9
        && !friendly_in_line_of_fire(state, rocket, dist)
    {
        cmd.shoot = true;
    }

    cmd
}

fn bullet_dodge(state: &GameStateView, rocket: &RocketView) -> Vec2 {
    let mut dodge = Vec2::ZERO;
    for b in &state.bullets {
        if b.team == state.my_team {
            continue;
        }
        let rel = dv2(state, b.position, rocket.position);
        let dist = rel.length();
        if !(20.0..450.0).contains(&dist) {
            continue;
        }
        let bv = b.velocity_vec2();
        let toward = rel.normalize_or_zero();
        if bv.dot(toward) < 140.0 {
            continue;
        }
        let perp = Vec2::new(-bv.y, bv.x).normalize_or_zero();
        let side = if perp.dot(rel) > 0.0 { 1.0 } else { -1.0 };
        dodge += perp * side * (220.0 * (1.0 - dist / 450.0));
    }
    dodge
}

fn friendly_in_line_of_fire(state: &GameStateView, rocket: &RocketView, target_dist: f32) -> bool {
    let forward = rocket.forward();
    let corridor = 35.0;
    let check_dist = target_dist.min(700.0);

    let station_delta = dv2(state, rocket.position, state.my_station.position);
    let along = station_delta.dot(forward);
    if along > 0.0 && along < check_dist {
        let perp = (station_delta - forward * along).length();
        if perp < corridor + 80.0 {
            return true;
        }
    }

    for ally in &state.my_rockets {
        if ally.id == rocket.id {
            continue;
        }
        let d = dv2(state, rocket.position, ally.position);
        let along = d.dot(forward);
        if along > 0.0 && along < check_dist && (d - forward * along).length() < corridor {
            return true;
        }
    }

    for tug in &state.my_tugs {
        let d = dv2(state, rocket.position, tug.position);
        let along = d.dot(forward);
        if along > 0.0 && along < check_dist && (d - forward * along).length() < corridor {
            return true;
        }
    }

    false
}

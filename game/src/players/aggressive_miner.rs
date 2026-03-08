//! AggressiveMiner AI — fast aggression with bullet deflection.
//!
//! Strategy:
//! - Build 2 tugs for economy, then all rockets (replace tugs if lost)
//! - Rockets attack enemy station immediately — no mining phase
//! - Defend only when threats are very close to our station
//! - Station tractor beams deflect incoming enemy bullets
//! - Tugs prefer closer asteroids and avoid danger

use std::collections::HashMap;
use bevy::math::Vec2;
use crate::api::*;
use crate::config::GameConfig;

#[derive(Clone, Copy, PartialEq)]
enum RocketRole {
    Attacking,
    Defending(EntityId),
}

pub struct AggressiveMinerAI {
    config: Option<GameConfig>,
    team: Option<Team>,
    rocket_roles: HashMap<EntityId, RocketRole>,
    tug_targets: HashMap<EntityId, EntityId>,
}

impl AggressiveMinerAI {
    pub fn new() -> Self {
        Self {
            config: None,
            team: None,
            rocket_roles: HashMap::new(),
            tug_targets: HashMap::new(),
        }
    }
}

impl PlayerAI for AggressiveMinerAI {
    fn name(&self) -> &str { "AggressiveMiner" }

    fn init(&mut self, config: &GameConfig, team: Team) {
        self.config = Some(config.clone());
        self.team = Some(team);
    }

    fn tick(&mut self, state: &GameStateView) -> Commands {
        let mut cmds = Commands::default();
        let num_tugs = state.my_tugs.len();

        // === ROCKETS ===
        // Clean dead rocket roles
        self.rocket_roles.retain(|rid, role| {
            if !state.my_rockets.iter().any(|r| r.id == *rid) { return false; }
            match role {
                RocketRole::Attacking => state.enemy_station.health > 0.0,
                RocketRole::Defending(eid) => state.enemy_rockets.iter().any(|r| r.id == *eid),
            }
        });

        // Find threats near our station
        let threats: Vec<&RocketView> = state.enemy_rockets.iter()
            .filter(|er| state.distance(er.position, state.my_station.position) < 3000.0)
            .collect();

        // Count how many rockets are near our station (rally zone) vs already attacking
        let rally_radius = 1500.0;
        let rockets_near_home: usize = state.my_rockets.iter()
            .filter(|r| state.distance(r.position, state.my_station.position) < rally_radius)
            .count();
        let rockets_attacking: usize = self.rocket_roles.values()
            .filter(|r| matches!(r, RocketRole::Attacking))
            .count();
        // Attack when we have 3+ rockets near home, OR some are already committed to attack
        let should_attack = rockets_near_home >= 3 || rockets_attacking > 0;

        // Rally point: ahead of our station toward enemy
        let station_pos = Vec2::new(state.my_station.position[0], state.my_station.position[1]);
        let to_enemy = delta_vec2(state, state.my_station.position, state.enemy_station.position);
        let rally_point = station_pos + to_enemy.normalize_or_zero() * 500.0;

        for rocket in &state.my_rockets {
            // Spawn clearance — fly away from station after spawning
            let dist_to_station = state.distance(rocket.position, state.my_station.position);
            if dist_to_station < 200.0 {
                let away = delta_vec2(state, state.my_station.position, rocket.position);
                let dir = away.normalize_or_zero();
                let cmd = fly_toward(rocket, dir * 400.0 + Vec2::new(rocket.position[0], rocket.position[1]));
                cmds.rockets.insert(rocket.id, cmd);
                continue;
            }

            // Defend if there's a nearby threat
            let closest_threat = if !threats.is_empty() {
                threats.iter()
                    .min_by(|a, b| {
                        let da = state.distance(rocket.position, a.position);
                        let db = state.distance(rocket.position, b.position);
                        da.partial_cmp(&db).unwrap()
                    })
                    .copied()
                    .filter(|t| state.distance(rocket.position, t.position) < 4000.0)
            } else {
                None
            };

            let cmd = if let Some(threat) = closest_threat {
                self.rocket_roles.insert(rocket.id, RocketRole::Defending(threat.id));
                fly_and_shoot(state, rocket, threat.position, threat.velocity, 200.0)
            } else if should_attack {
                // Attack! Engage enemy rockets en route, otherwise go for station
                self.rocket_roles.insert(rocket.id, RocketRole::Attacking);

                let nearby_enemy = state.enemy_rockets.iter()
                    .min_by(|a, b| {
                        let da = state.distance(rocket.position, a.position);
                        let db = state.distance(rocket.position, b.position);
                        da.partial_cmp(&db).unwrap()
                    })
                    .filter(|er| state.distance(rocket.position, er.position) < 800.0);

                if let Some(enemy) = nearby_enemy {
                    fly_and_shoot(state, rocket, enemy.position, enemy.velocity, 200.0)
                } else {
                    fly_and_shoot(
                        state, rocket,
                        state.enemy_station.position,
                        [0.0, 0.0],
                        350.0,
                    )
                }
            } else {
                // Rally near our station — wait for more rockets
                self.rocket_roles.remove(&rocket.id);
                fly_toward(rocket, rally_point)
            };

            cmds.rockets.insert(rocket.id, cmd);
        }

        // === TUGS ===
        let small_asteroids: Vec<&AsteroidView> = state.asteroids.iter()
            .filter(|a| a.tier <= 2)
            .collect();

        self.tug_targets.retain(|tug_id, target_id| {
            if !state.my_tugs.iter().any(|t| t.id == *tug_id) { return false; }
            state.asteroids.iter().any(|a| a.id == *target_id)
        });

        let mut claimed: Vec<EntityId> = Vec::new();
        for tug in &state.my_tugs {
            if let Some(c) = tug.carrying { claimed.push(c); }
        }
        for tid in self.tug_targets.values() { claimed.push(*tid); }

        let beam_radius = state.my_station.beam_radius;

        for tug in &state.my_tugs {
            let tug_vel = tug.velocity_vec2();
            let mut cmd = TugCommand::default();

            if tug.carrying.is_some() {
                let to_station = delta_vec2(state, tug.position, state.my_station.position);
                let station_dist = to_station.length();
                let desired_dir = to_station.normalize_or_zero();
                let drop_radius = beam_radius - 30.0;

                if station_dist < drop_radius {
                    cmd.beam_target = None;
                    self.tug_targets.remove(&tug.id);
                    let away = -desired_dir;
                    let perp = Vec2::new(-away.y, away.x);
                    let escape = (away + perp * 0.5).normalize_or_zero() * 100.0;
                    cmd.thrust = [escape.x, escape.y];
                } else if station_dist < drop_radius + 80.0 {
                    let frac = ((station_dist - drop_radius) / 80.0).clamp(0.0, 1.0);
                    let desired_vel = desired_dir * (frac * 120.0 + 20.0);
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = tug.carrying;
                } else {
                    let desired_vel = desired_dir * 150.0;
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = tug.carrying;
                }
            } else {
                // Find closest unclaimed small asteroid outside beam radius
                let target_id = self.tug_targets.get(&tug.id).and_then(|id| {
                    small_asteroids.iter().find(|a| a.id == *id).map(|a| a.id)
                });

                let target_id = target_id.unwrap_or_else(|| {
                    let best = small_asteroids.iter()
                        .filter(|a| {
                            !claimed.contains(&a.id)
                            && state.distance(a.position, state.my_station.position) > beam_radius
                        })
                        .min_by(|a, b| {
                            // Prefer asteroids that are closer to us AND closer to our station
                            // (less round-trip time)
                            let da = state.distance(tug.position, a.position)
                                + state.distance(a.position, state.my_station.position) * 0.5;
                            let db = state.distance(tug.position, b.position)
                                + state.distance(b.position, state.my_station.position) * 0.5;
                            da.partial_cmp(&db).unwrap()
                        });
                    if let Some(t) = best {
                        self.tug_targets.insert(tug.id, t.id);
                        claimed.push(t.id);
                        t.id
                    } else {
                        EntityId(0)
                    }
                });

                if let Some(target) = small_asteroids.iter().find(|a| a.id == target_id) {
                    let delta = delta_vec2(state, tug.position, target.position);
                    let dist = delta.length();
                    let desired_dir = delta.normalize_or_zero();
                    let desired_vel = desired_dir * 150.0;
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    if dist < 112.0 {
                        cmd.beam_target = Some(target.id);
                    }
                } else {
                    // No targets — patrol away from station
                    let to_station = delta_vec2(state, tug.position, state.my_station.position);
                    if to_station.length() < beam_radius + 100.0 {
                        let away = -to_station.normalize_or_zero();
                        let perp = Vec2::new(-away.y, away.x);
                        let escape = (away * 0.3 + perp * 0.7).normalize_or_zero() * 100.0;
                        cmd.thrust = [escape.x, escape.y];
                    }
                }
            }

            cmds.tugs.insert(tug.id, cmd);
        }

        // === STATION: DEFLECT INCOMING BULLETS ===
        // Use tractor beams to push away enemy bullets headed toward our station
        let station_pos = state.my_station.position;
        let beam_radius = state.my_station.beam_radius;
        let mut beam_cmds: Vec<BeamCommand> = Vec::new();

        for bullet in &state.bullets {
            if beam_cmds.len() >= 5 { break; } // Max 5 beams
            if bullet.team == state.my_team { continue; } // Ignore our own bullets

            let to_bullet = delta_vec2(state, station_pos, bullet.position);
            let dist = to_bullet.length();
            if dist > beam_radius { continue; } // Out of range

            // Check if bullet is heading toward us
            let bullet_vel = bullet.velocity_vec2();
            let toward_station = -to_bullet.normalize_or_zero();
            if bullet_vel.dot(toward_station) < 50.0 { continue; } // Not heading our way fast enough

            // Push it away (perpendicular to its velocity for maximum deflection)
            let perp = Vec2::new(-bullet_vel.y, bullet_vel.x).normalize_or_zero();
            beam_cmds.push(BeamCommand {
                target: bullet.id,
                force_direction: [perp.x, perp.y],
            });
        }

        if !beam_cmds.is_empty() {
            cmds.station.beam_targets = beam_cmds;
        }

        // === STATION: BUILD ORDER ===
        // 2 tugs for economy (replace if lost), then all rockets
        let minerals = state.my_station.resources;
        if state.my_station.build_progress.is_none() && state.my_station.build_queue_length == 0 {
            let want_tug = num_tugs < 2; // Always maintain 2 tugs

            let unit = if want_tug { UnitTypeView::Tug } else { UnitTypeView::Rocket };
            let cost = match unit {
                UnitTypeView::Rocket => 100.0,
                UnitTypeView::Tug => 75.0,
            };
            if minerals >= cost {
                cmds.station.build = Some(unit);
            }
        }

        cmds
    }
}

// === Shared helpers (same as example_ai) ===

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
    let target_v = Vec2::new(target_vel[0], target_vel[1]);

    let delta = delta_vec2(state, rocket.position, target_pos);
    let dist = delta.length();
    let to_target = delta.normalize_or_zero();

    // Lead the target: aim where it will be, not where it is
    let bullet_speed = 500.0;
    let relative_vel = target_v - rocket_vel;
    let intercept_time = if dist > 10.0 { dist / bullet_speed } else { 0.0 };
    let lead_pos = delta + target_v * intercept_time;
    let to_lead = lead_pos.normalize_or_zero();

    // Shoot if roughly aimed at target — be aggressive about shooting
    let aim_alignment = forward.dot(to_lead);
    if dist < 700.0
        && aim_alignment > 0.85
        && rocket.shoot_cooldown <= 0.0
        && !friendly_in_line_of_fire(state, rocket, dist)
    {
        cmd.shoot = true;
    }

    // Flight: always face the target and thrust toward standoff
    let dist_error = dist - standoff;
    let approach_speed = if dist_error > 200.0 {
        180.0
    } else if dist_error > 0.0 {
        40.0 + (dist_error / 200.0) * 140.0
    } else if dist_error > -50.0 {
        dist_error * 0.8
    } else {
        -80.0
    };

    let mut desired_vel = target_v + to_target * approach_speed;

    // Asteroid collision avoidance
    if let Some(avoidance) = asteroid_avoidance(state, rocket) {
        desired_vel += avoidance;
    }

    let delta_v = desired_vel - rocket_vel;
    let delta_v_mag = delta_v.length();

    // Always rotate toward the lead aim point for shooting
    let cross = forward.perp_dot(to_lead);
    cmd.rotation = cross.clamp(-1.0, 1.0);

    // Thrust based on velocity correction needs
    if delta_v_mag > 5.0 {
        let burn_dir = delta_v / delta_v_mag;
        let alignment = forward.dot(burn_dir);
        cmd.thrust = if alignment > 0.3 {
            (delta_v_mag / 40.0).clamp(0.2, 1.0)
        } else if alignment > -0.2 {
            0.1
        } else {
            0.0
        };
    }

    cmd
}

fn fly_toward(rocket: &RocketView, target_pos: Vec2) -> RocketCommand {
    let mut cmd = RocketCommand::default();
    let forward = rocket.forward();
    let rocket_pos = Vec2::new(rocket.position[0], rocket.position[1]);
    let delta = target_pos - rocket_pos;
    let dir = delta.normalize_or_zero();
    let cross = forward.perp_dot(dir);
    cmd.rotation = cross.clamp(-1.0, 1.0);
    if forward.dot(dir) > 0.5 { cmd.thrust = 1.0; }
    cmd
}

fn friendly_in_line_of_fire(state: &GameStateView, rocket: &RocketView, target_dist: f32) -> bool {
    let forward = rocket.forward();
    let rocket_pos = rocket.position;
    let check_dist = target_dist.min(600.0);
    let corridor = 30.0;

    // Station
    let delta = delta_vec2(state, rocket_pos, state.my_station.position);
    let along = delta.dot(forward);
    if along > 0.0 && along < check_dist {
        let perp = (delta - forward * along).length();
        if perp < corridor + 80.0 { return true; }
    }

    // Rockets
    for r in &state.my_rockets {
        if r.id == rocket.id { continue; }
        let delta = delta_vec2(state, rocket_pos, r.position);
        let along = delta.dot(forward);
        if along > 0.0 && along < check_dist {
            if (delta - forward * along).length() < corridor { return true; }
        }
    }

    // Tugs
    for t in &state.my_tugs {
        let delta = delta_vec2(state, rocket_pos, t.position);
        let along = delta.dot(forward);
        if along > 0.0 && along < check_dist {
            if (delta - forward * along).length() < corridor { return true; }
        }
    }

    false
}

/// Check if any asteroid is in our flight path and return an avoidance velocity nudge.
fn asteroid_avoidance(state: &GameStateView, rocket: &RocketView) -> Option<Vec2> {
    let rocket_vel = rocket.velocity_vec2();
    let speed = rocket_vel.length();
    if speed < 10.0 { return None; } // Not moving fast enough to worry

    let vel_dir = rocket_vel / speed;
    let look_ahead = 400.0; // How far ahead to check
    let clearance = 40.0; // Extra margin around asteroids

    let mut closest_threat: Option<(f32, Vec2)> = None; // (distance_along, perp_offset)

    for asteroid in &state.asteroids {
        let to_ast = delta_vec2(state, rocket.position, asteroid.position);
        let along = to_ast.dot(vel_dir);
        if along < 0.0 || along > look_ahead { continue; } // Behind us or too far

        let perp_offset = to_ast - vel_dir * along;
        let perp_dist = perp_offset.length();
        let danger_radius = asteroid.radius + clearance;

        if perp_dist < danger_radius {
            // This asteroid is in our path
            if closest_threat.is_none() || along < closest_threat.unwrap().0 {
                closest_threat = Some((along, perp_offset));
            }
        }
    }

    if let Some((along, perp_offset)) = closest_threat {
        // Steer perpendicular to our velocity, away from the asteroid center
        let urgency = 1.0 - (along / look_ahead); // More urgent when closer
        let avoid_dir = if perp_offset.length() > 1.0 {
            -perp_offset.normalize_or_zero() // Steer away from asteroid center
        } else {
            // Dead center — pick a side (perpendicular to velocity)
            Vec2::new(-vel_dir.y, vel_dir.x)
        };
        Some(avoid_dir * 200.0 * urgency)
    } else {
        None
    }
}

fn delta_vec2(state: &GameStateView, from: [f32; 2], to: [f32; 2]) -> Vec2 {
    let d = state.shortest_delta(from, to);
    Vec2::new(d[0], d[1])
}

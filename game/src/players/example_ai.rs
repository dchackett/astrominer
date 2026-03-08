//! Example AI that demonstrates how to use the PlayerAI trait.
//! Competitive AI that mines asteroids AND attacks the enemy:
//! - Rockets mine large asteroids early, then transition to attacking enemy station
//! - Tugs gather small asteroids and deliver to station
//! - Station builds units with adaptive ratio (more tugs early, more rockets later)

use std::collections::HashMap;
use bevy::math::Vec2;
use crate::api::*;
use crate::config::GameConfig;

/// Target types for rockets
#[derive(Clone, Copy, PartialEq)]
enum RocketTarget {
    Asteroid(EntityId),
    EnemyStation,
    EnemyRocket(EntityId),
}

pub struct ExampleAI {
    config: Option<GameConfig>,
    team: Option<Team>,
    rocket_targets: HashMap<EntityId, RocketTarget>,
    tug_targets: HashMap<EntityId, EntityId>,
    builds_done: u32,
}

impl ExampleAI {
    pub fn new() -> Self {
        Self {
            config: None,
            team: None,
            rocket_targets: HashMap::new(),
            tug_targets: HashMap::new(),
            builds_done: 0,
        }
    }
}

impl PlayerAI for ExampleAI {
    fn name(&self) -> &str {
        "ExampleAI"
    }

    fn init(&mut self, config: &GameConfig, team: Team) {
        self.config = Some(config.clone());
        self.team = Some(team);
    }

    fn tick(&mut self, state: &GameStateView) -> Commands {
        let mut cmds = Commands::default();

        let large_asteroids: Vec<&AsteroidView> = state.asteroids.iter()
            .filter(|a| a.tier >= 3)
            .collect();

        let num_rockets = state.my_rockets.len();
        let num_tugs = state.my_tugs.len();

        // Decide how many rockets should attack vs mine
        // Early game: all rockets mine. As asteroids deplete or we have enough rockets, attack.
        let attack_rocket_count = if large_asteroids.is_empty() {
            num_rockets // No asteroids left, everyone attacks
        } else if num_rockets > 2 {
            num_rockets / 2 // Send half to attack once we have a few
        } else {
            0
        };

        // Clean up dead targets
        self.rocket_targets.retain(|rocket_id, target| {
            // Check rocket still exists
            if !state.my_rockets.iter().any(|r| r.id == *rocket_id) { return false; }
            match target {
                RocketTarget::Asteroid(id) => {
                    state.asteroids.iter().any(|a| a.id == *id && a.tier >= 3)
                }
                RocketTarget::EnemyStation => state.enemy_station.health > 0.0,
                RocketTarget::EnemyRocket(id) => {
                    state.enemy_rockets.iter().any(|r| r.id == *id)
                }
            }
        });

        // Count current attackers
        let current_attackers = self.rocket_targets.values()
            .filter(|t| matches!(t, RocketTarget::EnemyStation | RocketTarget::EnemyRocket(_)))
            .count();

        // Assign rockets
        for rocket in &state.my_rockets {
            // First priority: if we just spawned and are near our station, fly away
            let dist_to_own_station = state.distance(rocket.position, state.my_station.position);
            if dist_to_own_station < 200.0 {
                // Just spawned — get clear of our station before doing anything else
                let away = delta_vec2(state, state.my_station.position, rocket.position);
                let away_dir = away.normalize_or_zero();
                let cmd = fly_toward(rocket, away_dir * 300.0 + Vec2::new(rocket.position[0], rocket.position[1]));
                cmds.rockets.insert(rocket.id, cmd);
                continue;
            }

            // Decide role: mining or attacking
            let should_attack = if let Some(target) = self.rocket_targets.get(&rocket.id) {
                matches!(target, RocketTarget::EnemyStation | RocketTarget::EnemyRocket(_))
            } else {
                current_attackers < attack_rocket_count
            };

            let cmd;

            if should_attack {
                // ATTACK MODE: target enemy station or nearby enemy rockets
                let nearby_threat = state.enemy_rockets.iter().find(|er| {
                    let to_my_station = state.distance(er.position, state.my_station.position);
                    to_my_station < 800.0
                });

                if let Some(threat) = nearby_threat {
                    self.rocket_targets.insert(rocket.id, RocketTarget::EnemyRocket(threat.id));
                    cmd = fly_and_shoot(state, rocket, threat.position, threat.velocity, 200.0);
                } else {
                    self.rocket_targets.insert(rocket.id, RocketTarget::EnemyStation);
                    cmd = fly_and_shoot(
                        state, rocket,
                        state.enemy_station.position,
                        [0.0, 0.0],
                        300.0,
                    );
                }
            } else {
                // MINE MODE: find and shoot large asteroids
                let target = self.rocket_targets.get(&rocket.id)
                    .and_then(|t| match t {
                        RocketTarget::Asteroid(id) => state.asteroids.iter().find(|a| a.id == *id),
                        _ => None,
                    })
                    .or_else(|| {
                        let best = large_asteroids.iter()
                            .min_by(|a, b| {
                                let da = state.distance(rocket.position, a.position);
                                let db = state.distance(rocket.position, b.position);
                                da.partial_cmp(&db).unwrap()
                            });
                        if let Some(&&ref t) = best {
                            self.rocket_targets.insert(rocket.id, RocketTarget::Asteroid(t.id));
                        }
                        best.copied()
                    });

                if let Some(target) = target {
                    cmd = fly_and_shoot(
                        state, rocket,
                        target.position, target.velocity,
                        250.0 + target.radius,
                    );
                } else {
                    self.rocket_targets.insert(rocket.id, RocketTarget::EnemyStation);
                    cmd = fly_and_shoot(
                        state, rocket,
                        state.enemy_station.position,
                        [0.0, 0.0],
                        300.0,
                    );
                }
            }

            cmds.rockets.insert(rocket.id, cmd);
        }

        // --- Tugs: gather small asteroids ---
        let small_asteroids: Vec<&AsteroidView> = state.asteroids.iter()
            .filter(|a| a.tier <= 2)
            .collect();

        // Clean up dead targets
        self.tug_targets.retain(|tug_id, target_id| {
            // Remove if tug is gone
            if !state.my_tugs.iter().any(|t| t.id == *tug_id) { return false; }
            // Remove if asteroid is gone
            state.asteroids.iter().any(|a| a.id == *target_id)
        });

        // Track which asteroids are already claimed by a tug (carrying or targeted)
        let mut claimed_asteroids: Vec<EntityId> = Vec::new();
        for tug in &state.my_tugs {
            if let Some(carrying_id) = tug.carrying {
                claimed_asteroids.push(carrying_id);
            }
        }
        for target_id in self.tug_targets.values() {
            claimed_asteroids.push(*target_id);
        }

        let beam_radius = state.my_station.beam_radius;

        for tug in &state.my_tugs {
            let tug_vel = tug.velocity_vec2();
            let mut cmd = TugCommand::default();

            if tug.carrying.is_some() {
                let to_station = delta_vec2(state, tug.position, state.my_station.position);
                let station_dist = to_station.length();
                let desired_dir = to_station.normalize_or_zero();

                // Release inside the station beam radius so the station can grab it
                let drop_radius = beam_radius - 30.0;
                if station_dist < drop_radius {
                    // We're inside the beam zone — release the asteroid and get out
                    cmd.beam_target = None;
                    self.tug_targets.remove(&tug.id);
                    // Fly away from station so we don't get sucked in
                    let away = -desired_dir;
                    let perp = Vec2::new(-away.y, away.x);
                    let escape = (away + perp * 0.5).normalize_or_zero() * 100.0;
                    cmd.thrust = [escape.x, escape.y];
                } else if station_dist < drop_radius + 80.0 {
                    // Approaching drop zone — slow down
                    let approach_fraction = ((station_dist - drop_radius) / 80.0).clamp(0.0, 1.0);
                    let desired_speed = approach_fraction * 120.0 + 20.0;
                    let desired_vel = desired_dir * desired_speed;
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = tug.carrying;
                } else {
                    // Far from station — fly toward it
                    let desired_vel = desired_dir * 150.0;
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = tug.carrying;
                }
            } else {
                // Not carrying — find an unclaimed asteroid to pick up
                let target_id = self.tug_targets.get(&tug.id).and_then(|id| {
                    small_asteroids.iter().find(|a| a.id == *id).map(|a| a.id)
                });

                let target_id = target_id.unwrap_or_else(|| {
                    let best = small_asteroids.iter()
                        .filter(|a| {
                            // Not already claimed by another tug
                            !claimed_asteroids.contains(&a.id)
                            // Not already inside station beam (station will grab it)
                            && state.distance(a.position, state.my_station.position) > beam_radius
                        })
                        .min_by(|a, b| {
                            let da = state.distance(tug.position, a.position);
                            let db = state.distance(tug.position, b.position);
                            da.partial_cmp(&db).unwrap()
                        });
                    if let Some(t) = best {
                        self.tug_targets.insert(tug.id, t.id);
                        claimed_asteroids.push(t.id);
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
                    // No targets — orbit away from station
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

        // --- Station: adaptive build ---
        let minerals = state.my_station.resources;
        if state.my_station.build_progress.is_none() && state.my_station.build_queue_length == 0 {
            // Early game (few units): prioritize 2 tugs then rockets
            // Later: mostly rockets with occasional tug
            let want_tug = if num_tugs == 0 {
                true
            } else if self.builds_done < 4 {
                num_tugs < 2 // Get at least 2 tugs early
            } else {
                num_tugs < num_rockets / 2 // Keep ~1:2 tug:rocket ratio
            };

            let unit = if want_tug { UnitTypeView::Tug } else { UnitTypeView::Rocket };
            let cost = match unit {
                UnitTypeView::Rocket => 100.0,
                UnitTypeView::Tug => 75.0,
            };
            if minerals >= cost {
                cmds.station.build = Some(unit);
                self.builds_done += 1;
            }
        }

        cmds
    }
}

/// Fly toward a target, maintaining standoff distance.
/// Always checks for opportunistic shots regardless of flight phase.
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

    // --- Always check for opportunistic shots first ---
    // If we happen to be aimed at the target and in range, shoot regardless of what
    // flight maneuver we're doing
    let aim_alignment = forward.dot(to_target);
    if dist < 600.0
        && aim_alignment > 0.9
        && rocket.shoot_cooldown <= 0.0
        && !friendly_in_line_of_fire(state, rocket, dist)
    {
        cmd.shoot = true;
    }

    // --- Flight control: approach standoff distance ---
    // Use a soft gradient instead of hard zones
    let dist_error = dist - standoff;
    let approach_speed = if dist_error > 200.0 {
        160.0  // Far away, full approach
    } else if dist_error > 0.0 {
        // Ramp down as we approach standoff
        40.0 + (dist_error / 200.0) * 120.0
    } else if dist_error > -100.0 {
        // Slightly inside standoff, gentle drift outward
        dist_error * 0.5
    } else {
        // Way too close, back off
        -100.0
    };

    let desired_vel = target_v + to_target * approach_speed;
    let delta_v = desired_vel - rocket_vel;
    let delta_v_mag = delta_v.length();

    if delta_v_mag > 5.0 {
        let burn_dir = delta_v / delta_v_mag;

        // Blend between aiming at target and burning for velocity correction
        // Prioritize aiming when we're near standoff and almost aimed
        let aim_priority = if dist_error.abs() < 100.0 && aim_alignment > 0.5 {
            0.7 // Mostly aim at target
        } else {
            0.0 // Mostly correct velocity
        };

        let blended_dir = (burn_dir * (1.0 - aim_priority) + to_target * aim_priority).normalize_or_zero();
        let cross = forward.perp_dot(blended_dir);
        cmd.rotation = cross.clamp(-1.0, 1.0);

        let alignment = forward.dot(burn_dir);
        cmd.thrust = if alignment > 0.6 {
            (delta_v_mag / 50.0).clamp(0.3, 1.0) // Proportional thrust
        } else if alignment > 0.2 {
            0.2
        } else {
            0.0
        };
    } else {
        // Velocity is close to desired — just aim at target
        let cross = forward.perp_dot(to_target);
        cmd.rotation = cross.clamp(-1.0, 1.0);
    }

    cmd
}

/// Simple "fly toward a point" for clearing station after spawn.
fn fly_toward(rocket: &RocketView, target_pos: Vec2) -> RocketCommand {
    let mut cmd = RocketCommand::default();
    let forward = rocket.forward();
    let rocket_pos = Vec2::new(rocket.position[0], rocket.position[1]);
    let delta = target_pos - rocket_pos;
    let dir = delta.normalize_or_zero();
    let cross = forward.perp_dot(dir);
    cmd.rotation = cross.clamp(-1.0, 1.0);
    if forward.dot(dir) > 0.5 {
        cmd.thrust = 1.0;
    }
    cmd
}

/// Check if any friendly unit (station, rocket, tug) is in the bullet's path.
/// Uses a simple cone/corridor check along the rocket's forward direction.
fn friendly_in_line_of_fire(state: &GameStateView, rocket: &RocketView, target_dist: f32) -> bool {
    let forward = rocket.forward();
    let rocket_pos = rocket.position;
    let check_dist = target_dist.min(600.0); // Don't check beyond target or max range
    let corridor_width = 30.0; // How wide a corridor to check for friendlies

    // Check own station
    {
        let delta = delta_vec2(state, rocket_pos, state.my_station.position);
        let along = delta.dot(forward);
        if along > 0.0 && along < check_dist {
            let perp = (delta - forward * along).length();
            if perp < corridor_width + 80.0 {
                // Station is big, use generous radius
                return true;
            }
        }
    }

    // Check own rockets
    for r in &state.my_rockets {
        if r.id == rocket.id { continue; }
        let delta = delta_vec2(state, rocket_pos, r.position);
        let along = delta.dot(forward);
        if along > 0.0 && along < check_dist {
            let perp = (delta - forward * along).length();
            if perp < corridor_width {
                return true;
            }
        }
    }

    // Check own tugs
    for t in &state.my_tugs {
        let delta = delta_vec2(state, rocket_pos, t.position);
        let along = delta.dot(forward);
        if along > 0.0 && along < check_dist {
            let perp = (delta - forward * along).length();
            if perp < corridor_width {
                return true;
            }
        }
    }

    false
}

fn delta_vec2(state: &GameStateView, from: [f32; 2], to: [f32; 2]) -> Vec2 {
    let d = state.shortest_delta(from, to);
    Vec2::new(d[0], d[1])
}

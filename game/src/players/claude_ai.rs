//! ClaudeAI v2 — aggressive economy + surgical strikes.
//!
//! v2 changes: tighter defense radius, hunt enemy tugs, bullet dodging,
//! faster rocket production, station self-repair, wider shooting range.

use crate::api::*;
use crate::config::GameConfig;
use bevy::math::Vec2;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum RocketRole {
    Mining(EntityId),
    Attacking,
    HuntingTug(EntityId),
    Defending(EntityId),
}

pub struct ClaudeAI {
    config: Option<GameConfig>,
    team: Option<Team>,
    rocket_roles: HashMap<EntityId, RocketRole>,
    tug_targets: HashMap<EntityId, EntityId>,
    total_builds: u32,
}

impl ClaudeAI {
    pub fn new() -> Self {
        Self {
            config: None,
            team: None,
            rocket_roles: HashMap::new(),
            tug_targets: HashMap::new(),
            total_builds: 0,
        }
    }
}

impl PlayerAI for ClaudeAI {
    fn name(&self) -> &str {
        "ClaudeAI"
    }

    fn init(&mut self, config: &GameConfig, team: Team) {
        self.config = Some(config.clone());
        self.team = Some(team);
    }

    fn tick(&mut self, state: &GameStateView) -> Commands {
        let mut cmds = Commands::default();

        let num_rockets = state.my_rockets.len();
        let num_tugs = state.my_tugs.len();

        // === PHASE DETECTION ===
        let large_asteroids: Vec<&AsteroidView> =
            state.asteroids.iter().filter(|a| a.tier >= 3).collect();
        let small_asteroids: Vec<&AsteroidView> =
            state.asteroids.iter().filter(|a| a.tier <= 2).collect();

        let early_game = state.tick < 600;

        // Only defend against rockets very close to our station
        let station_threats: Vec<&RocketView> = state
            .enemy_rockets
            .iter()
            .filter(|er| state.distance(er.position, state.my_station.position) < 1500.0)
            .collect();

        // === CLEAN UP DEAD TARGETS ===
        self.rocket_roles.retain(|rid, role| {
            if !state.my_rockets.iter().any(|r| r.id == *rid) {
                return false;
            }
            match role {
                RocketRole::Mining(aid) => {
                    state.asteroids.iter().any(|a| a.id == *aid && a.tier >= 3)
                }
                RocketRole::Attacking => state.enemy_station.health > 0.0,
                RocketRole::HuntingTug(tid) => state.enemy_tugs.iter().any(|t| t.id == *tid),
                RocketRole::Defending(eid) => state.enemy_rockets.iter().any(|r| r.id == *eid),
            }
        });

        // === ROCKET STRATEGY ===
        let miners_needed = if large_asteroids.is_empty() || num_rockets <= 1 {
            0
        } else if early_game {
            (num_rockets / 2).max(1)
        } else {
            0
        };

        // Assign 1 rocket to hunt enemy tugs if they have any and we have spare rockets
        let tug_hunter_needed = if !state.enemy_tugs.is_empty() && num_rockets >= 4 {
            1
        } else {
            0
        };

        let current_miners = self
            .rocket_roles
            .values()
            .filter(|r| matches!(r, RocketRole::Mining(_)))
            .count();
        let current_tug_hunters = self
            .rocket_roles
            .values()
            .filter(|r| matches!(r, RocketRole::HuntingTug(_)))
            .count();

        // Focus fire: pick weakest enemy rocket near their station (where our attackers go)
        let focus_target: Option<&RocketView> = if !state.enemy_rockets.is_empty() {
            // Prefer weakest enemy near the enemy station (our attack target)
            state.enemy_rockets.iter().min_by(|a, b| {
                // Score: low health + near enemy station = high priority
                let score_a = a.health
                    + state.distance(a.position, state.enemy_station.position) * 0.05;
                let score_b = b.health
                    + state.distance(b.position, state.enemy_station.position) * 0.05;
                score_a.partial_cmp(&score_b).unwrap()
            })
        } else {
            None
        };

        for rocket in &state.my_rockets {
            let dist_to_station = state.distance(rocket.position, state.my_station.position);

            // Spawn clearance — fly toward enemy, not just "away"
            if dist_to_station < 150.0 {
                let toward_enemy = dv2(state, rocket.position, state.enemy_station.position);
                let target = toward_enemy.normalize_or_zero() * 400.0
                    + Vec2::new(rocket.position[0], rocket.position[1]);
                cmds.rockets.insert(rocket.id, fly_toward(rocket, target));
                continue;
            }

            // Priority 1: Defend against threats very close to our station
            if !station_threats.is_empty() {
                // Only divert rockets that are near our station themselves
                if dist_to_station < 2500.0 {
                    let closest_threat = station_threats
                        .iter()
                        .min_by(|a, b| {
                            let da = state.distance(rocket.position, a.position);
                            let db = state.distance(rocket.position, b.position);
                            da.partial_cmp(&db).unwrap()
                        })
                        .copied();

                    if let Some(threat) = closest_threat {
                        self.rocket_roles
                            .insert(rocket.id, RocketRole::Defending(threat.id));
                        let cmd =
                            fly_and_shoot(state, rocket, threat.position, threat.velocity, 160.0);
                        cmds.rockets.insert(rocket.id, cmd);
                        continue;
                    }
                }
            }

            // Priority 2: Mine if needed
            let role = self.rocket_roles.get(&rocket.id);
            let should_mine = match role {
                Some(RocketRole::Mining(_)) => true,
                _ => current_miners < miners_needed && !large_asteroids.is_empty(),
            };

            if should_mine {
                let target_ast = match role {
                    Some(RocketRole::Mining(aid)) => {
                        state.asteroids.iter().find(|a| a.id == *aid && a.tier >= 3)
                    }
                    _ => None,
                }
                .or_else(|| {
                    // Prefer mid-tier (3-4) asteroids — they fracture into more gatherable pieces
                    large_asteroids
                        .iter()
                        .min_by(|a, b| {
                            let tier_bonus_a = if a.tier <= 4 { 0.0 } else { 500.0 };
                            let tier_bonus_b = if b.tier <= 4 { 0.0 } else { 500.0 };
                            let da =
                                state.distance(rocket.position, a.position) + tier_bonus_a;
                            let db =
                                state.distance(rocket.position, b.position) + tier_bonus_b;
                            da.partial_cmp(&db).unwrap()
                        })
                        .copied()
                });

                if let Some(ast) = target_ast {
                    self.rocket_roles
                        .insert(rocket.id, RocketRole::Mining(ast.id));
                    let cmd = fly_and_shoot(
                        state,
                        rocket,
                        ast.position,
                        ast.velocity,
                        180.0 + ast.radius,
                    );
                    cmds.rockets.insert(rocket.id, cmd);
                    continue;
                }
            }

            // Priority 3: Hunt enemy tugs (economic warfare)
            let should_hunt = match role {
                Some(RocketRole::HuntingTug(_)) => true,
                _ => current_tug_hunters < tug_hunter_needed,
            };

            if should_hunt && !state.enemy_tugs.is_empty() {
                let target_tug = match role {
                    Some(RocketRole::HuntingTug(tid)) => {
                        state.enemy_tugs.iter().find(|t| t.id == *tid)
                    }
                    _ => None,
                }
                .or_else(|| {
                    state.enemy_tugs.iter().min_by(|a, b| {
                        let da = state.distance(rocket.position, a.position);
                        let db = state.distance(rocket.position, b.position);
                        da.partial_cmp(&db).unwrap()
                    })
                });

                if let Some(tug) = target_tug {
                    self.rocket_roles
                        .insert(rocket.id, RocketRole::HuntingTug(tug.id));
                    let cmd =
                        fly_and_shoot(state, rocket, tug.position, tug.velocity, 120.0);
                    cmds.rockets.insert(rocket.id, cmd);
                    continue;
                }
            }

            // Priority 4: Attack — focus fire on enemies near our path, then station
            self.rocket_roles.insert(rocket.id, RocketRole::Attacking);

            // Engage nearby enemy rockets opportunistically (wider range)
            let nearby_enemy = state
                .enemy_rockets
                .iter()
                .min_by(|a, b| {
                    let da = state.distance(rocket.position, a.position);
                    let db = state.distance(rocket.position, b.position);
                    da.partial_cmp(&db).unwrap()
                })
                .filter(|er| state.distance(rocket.position, er.position) < 800.0);

            let cmd = if let Some(enemy) = nearby_enemy {
                fly_and_shoot(state, rocket, enemy.position, enemy.velocity, 170.0)
            } else if let Some(focus) = focus_target {
                let dist_to_focus = state.distance(rocket.position, focus.position);
                if dist_to_focus < 2500.0 {
                    fly_and_shoot(state, rocket, focus.position, focus.velocity, 170.0)
                } else {
                    fly_and_shoot(
                        state,
                        rocket,
                        state.enemy_station.position,
                        [0.0, 0.0],
                        250.0,
                    )
                }
            } else {
                fly_and_shoot(
                    state,
                    rocket,
                    state.enemy_station.position,
                    [0.0, 0.0],
                    250.0,
                )
            };
            cmds.rockets.insert(rocket.id, cmd);
        }

        // === TUGS ===
        self.tug_targets.retain(|tug_id, target_id| {
            if !state.my_tugs.iter().any(|t| t.id == *tug_id) {
                return false;
            }
            state.asteroids.iter().any(|a| a.id == *target_id)
        });

        let mut claimed: Vec<EntityId> = Vec::new();
        for tug in &state.my_tugs {
            if let Some(c) = tug.carrying {
                claimed.push(c);
            }
        }
        for tid in self.tug_targets.values() {
            claimed.push(*tid);
        }

        let beam_radius = state.my_station.beam_radius;

        for tug in &state.my_tugs {
            let tug_vel = tug.velocity_vec2();
            let mut cmd = TugCommand::default();

            if tug.carrying.is_some() {
                let to_station = dv2(state, tug.position, state.my_station.position);
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
                } else if station_dist < drop_radius + 100.0 {
                    let frac = ((station_dist - drop_radius) / 100.0).clamp(0.0, 1.0);
                    let desired_vel = desired_dir * (frac * 130.0 + 20.0);
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = tug.carrying;
                } else {
                    let desired_vel = desired_dir * 160.0;
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = tug.carrying;
                }
            } else {
                // Avoid enemy rockets — flee if one is within 500 units
                let danger = state.enemy_rockets.iter().any(|er| {
                    state.distance(tug.position, er.position) < 500.0
                });
                if danger {
                    // Run to our station
                    let to_station = dv2(state, tug.position, state.my_station.position);
                    let desired_vel = to_station.normalize_or_zero() * 100.0;
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmds.tugs.insert(tug.id, cmd);
                    continue;
                }

                let target_id = self
                    .tug_targets
                    .get(&tug.id)
                    .and_then(|id| small_asteroids.iter().find(|a| a.id == *id).map(|a| a.id));

                let target_id = target_id.unwrap_or_else(|| {
                    let best = small_asteroids
                        .iter()
                        .filter(|a| {
                            !claimed.contains(&a.id)
                                && state.distance(a.position, state.my_station.position)
                                    > beam_radius
                        })
                        .min_by(|a, b| {
                            let sa = state.distance(tug.position, a.position)
                                + state.distance(a.position, state.my_station.position) * 0.6;
                            let sb = state.distance(tug.position, b.position)
                                + state.distance(b.position, state.my_station.position) * 0.6;
                            sa.partial_cmp(&sb).unwrap()
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
                    let delta = dv2(state, tug.position, target.position);
                    let dist = delta.length();
                    let desired_dir = delta.normalize_or_zero();
                    let desired_vel = desired_dir * 155.0;
                    let dv = desired_vel - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    if dist < 112.0 {
                        cmd.beam_target = Some(target.id);
                    }
                } else {
                    let to_station = dv2(state, tug.position, state.my_station.position);
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

        // === STATION ===

        // Bullet deflection — prioritize bullets closest to hitting station
        let mut beam_cmds: Vec<BeamCommand> = Vec::new();
        let mut incoming_bullets: Vec<(&BulletView, f32)> = state
            .bullets
            .iter()
            .filter(|b| b.team != state.my_team)
            .filter_map(|b| {
                let to_bullet = dv2(state, state.my_station.position, b.position);
                let dist = to_bullet.length();
                if dist > beam_radius {
                    return None;
                }
                let bv = b.velocity_vec2();
                let toward = -to_bullet.normalize_or_zero();
                if bv.dot(toward) < 50.0 {
                    return None;
                }
                Some((b, dist))
            })
            .collect();
        // Sort by distance — closest bullets first (most urgent)
        incoming_bullets.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        for (bullet, _dist) in &incoming_bullets {
            if beam_cmds.len() >= 5 {
                break;
            }
            let bv = bullet.velocity_vec2();
            let perp = Vec2::new(-bv.y, bv.x).normalize_or_zero();
            beam_cmds.push(BeamCommand {
                target: bullet.id,
                force_direction: [perp.x, perp.y],
            });
        }

        // Use remaining beams to pull in small asteroids
        if beam_cmds.len() < 5 {
            let mut pullable: Vec<&AsteroidView> = state
                .asteroids
                .iter()
                .filter(|a| {
                    a.tier <= 2
                        && state.distance(a.position, state.my_station.position) <= beam_radius
                })
                .collect();
            pullable.sort_by(|a, b| {
                let da = state.distance(a.position, state.my_station.position);
                let db = state.distance(b.position, state.my_station.position);
                da.partial_cmp(&db).unwrap()
            });

            for ast in pullable {
                if beam_cmds.len() >= 5 {
                    break;
                }
                let to_station = dv2(state, ast.position, state.my_station.position);
                let dir = to_station.normalize_or_zero();
                beam_cmds.push(BeamCommand {
                    target: ast.id,
                    force_direction: [dir.x, dir.y],
                });
            }
        }

        if !beam_cmds.is_empty() {
            cmds.station.beam_targets = beam_cmds;
        }

        // Repair damaged units near station
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
                *ratio < 0.8 && state.distance(state.my_station.position, *pos) <= beam_radius
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        if let Some((id, _, _)) = repair_candidate {
            cmds.station.repair_target = Some(id);
        }

        // Build order: 2 tugs early for economy, then pure rockets
        let minerals = state.my_station.resources;
        if state.my_station.build_progress.is_none() && state.my_station.build_queue_length == 0 {
            let want_tug = if num_tugs == 0 {
                true
            } else if self.total_builds < 4 {
                num_tugs < 2
            } else {
                num_tugs < 1 // Only replace if we have zero
            };

            let unit = if want_tug {
                UnitTypeView::Tug
            } else {
                UnitTypeView::Rocket
            };
            let cost = match unit {
                UnitTypeView::Rocket => 50.0,
                UnitTypeView::Tug => 37.5,
            };
            if minerals >= cost {
                cmds.station.build = Some(unit);
                self.total_builds += 1;
            }
        }

        cmds
    }
}

// === Helper functions ===

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
    let target_v = Vec2::new(target_vel[0], target_vel[1]);

    let delta = dv2(state, rocket.position, target_pos);
    let dist = delta.length();
    let to_target = delta.normalize_or_zero();

    // Lead the target with proper relative velocity calculation
    let bullet_speed = 500.0;
    let closing_speed = -(target_v - rocket_vel).dot(to_target);
    let effective_bullet_speed = (bullet_speed + closing_speed.max(0.0)).max(60.0);
    let intercept_time = if dist > 10.0 {
        dist / effective_bullet_speed
    } else {
        0.0
    };
    let lead_pos = delta + target_v * intercept_time;
    let to_lead = lead_pos.normalize_or_zero();

    // Shoot if aimed well — wider range for more aggression
    let aim_alignment = forward.dot(to_lead);
    if dist < 900.0
        && aim_alignment > 0.9
        && rocket.shoot_cooldown <= 0.0
        && !friendly_in_line_of_fire(state, rocket, dist)
    {
        cmd.shoot = true;
    }

    // Flight control
    let dist_error = dist - standoff;
    let approach_speed = if dist_error > 200.0 {
        220.0
    } else if dist_error > 0.0 {
        60.0 + (dist_error / 200.0) * 160.0
    } else if dist_error > -60.0 {
        dist_error * 1.0
    } else {
        -120.0
    };

    let mut desired_vel = target_v + to_target * approach_speed;

    // Lateral strafing at standoff — alternate based on tick for unpredictability
    if dist_error.abs() < 100.0 && dist < 600.0 {
        let perp = Vec2::new(-to_target.y, to_target.x);
        let phase = (state.tick as f32 / 60.0 + rocket.id.0 as f32 * 1.7).sin();
        desired_vel += perp * phase * 50.0;
    }

    // Bullet dodging — check if any enemy bullet is heading toward us
    for bullet in &state.bullets {
        if bullet.team == state.my_team {
            continue;
        }
        let to_bullet = dv2(state, rocket.position, bullet.position);
        let b_dist = to_bullet.length();
        if b_dist > 300.0 {
            continue;
        }
        let bv = bullet.velocity_vec2();
        // Is bullet heading toward us?
        let toward_us = -to_bullet.normalize_or_zero();
        if bv.dot(toward_us) > 200.0 {
            // Check if we're in the bullet's path
            let bullet_dir = bv.normalize_or_zero();
            let perp_dist = to_bullet.dot(Vec2::new(-bullet_dir.y, bullet_dir.x)).abs();
            if perp_dist < 30.0 {
                // Dodge perpendicular to bullet velocity
                let dodge = Vec2::new(-bullet_dir.y, bullet_dir.x);
                let dodge_sign = if to_bullet.dot(dodge) > 0.0 { -1.0 } else { 1.0 };
                desired_vel += dodge * dodge_sign * 150.0;
                break; // Only dodge most threatening bullet
            }
        }
    }

    // Asteroid avoidance
    if let Some(avoidance) = asteroid_avoidance(state, rocket) {
        desired_vel += avoidance;
    }

    let delta_v = desired_vel - rocket_vel;
    let delta_v_mag = delta_v.length();

    // Rotate toward lead aim point
    let cross = forward.perp_dot(to_lead);
    cmd.rotation = cross.clamp(-1.0, 1.0);

    // Thrust
    if delta_v_mag > 5.0 {
        let burn_dir = delta_v / delta_v_mag;
        let alignment = forward.dot(burn_dir);
        cmd.thrust = if alignment > 0.3 {
            (delta_v_mag / 30.0).clamp(0.2, 1.0)
        } else if alignment > -0.2 {
            0.15
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
    if forward.dot(dir) > 0.3 {
        cmd.thrust = 1.0;
    }
    cmd
}

fn friendly_in_line_of_fire(state: &GameStateView, rocket: &RocketView, target_dist: f32) -> bool {
    let forward = rocket.forward();
    let rocket_pos = rocket.position;
    let check_dist = target_dist.min(700.0);
    let corridor = 35.0;

    // Station
    let delta = dv2(state, rocket_pos, state.my_station.position);
    let along = delta.dot(forward);
    if along > 0.0 && along < check_dist {
        let perp = (delta - forward * along).length();
        if perp < corridor + 80.0 {
            return true;
        }
    }

    // Rockets
    for r in &state.my_rockets {
        if r.id == rocket.id {
            continue;
        }
        let delta = dv2(state, rocket_pos, r.position);
        let along = delta.dot(forward);
        if along > 0.0 && along < check_dist {
            if (delta - forward * along).length() < corridor {
                return true;
            }
        }
    }

    // Tugs
    for t in &state.my_tugs {
        let delta = dv2(state, rocket_pos, t.position);
        let along = delta.dot(forward);
        if along > 0.0 && along < check_dist {
            if (delta - forward * along).length() < corridor {
                return true;
            }
        }
    }

    false
}

fn asteroid_avoidance(state: &GameStateView, rocket: &RocketView) -> Option<Vec2> {
    let rocket_vel = rocket.velocity_vec2();
    let speed = rocket_vel.length();
    if speed < 10.0 {
        return None;
    }

    let vel_dir = rocket_vel / speed;
    let look_ahead = 400.0;
    let clearance = 45.0;
    let mut closest_threat: Option<(f32, Vec2)> = None;

    for asteroid in &state.asteroids {
        let to_ast = dv2(state, rocket.position, asteroid.position);
        let along = to_ast.dot(vel_dir);
        if along < 0.0 || along > look_ahead {
            continue;
        }

        let perp_offset = to_ast - vel_dir * along;
        let perp_dist = perp_offset.length();
        let danger_radius = asteroid.radius + clearance;

        if perp_dist < danger_radius {
            if closest_threat.is_none() || along < closest_threat.unwrap().0 {
                closest_threat = Some((along, perp_offset));
            }
        }
    }

    if let Some((along, perp_offset)) = closest_threat {
        let urgency = 1.0 - (along / look_ahead);
        let avoid_dir = if perp_offset.length() > 1.0 {
            -perp_offset.normalize_or_zero()
        } else {
            Vec2::new(-vel_dir.y, vel_dir.x)
        };
        Some(avoid_dir * 220.0 * urgency)
    } else {
        None
    }
}

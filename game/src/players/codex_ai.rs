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
        let large_asteroids: Vec<&AsteroidView> =
            state.asteroids.iter().filter(|a| a.tier >= 3).collect();
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
        let severe_station_threat = state
            .enemy_rockets
            .iter()
            .filter(|r| state.distance(r.position, state.my_station.position) < 2000.0)
            .count()
            >= 2;

        if state.my_station.build_progress.is_none() && state.my_station.build_queue_length == 0 {
            let late_econ_tug = state.tick >= 2200
                && state.my_tugs.len() < 3
                && state.my_station.resources > 110.0
                && state.my_rockets.len() >= 5
                && state.enemy_tugs.len() >= state.my_tugs.len();
            let need_tugs = state.my_tugs.is_empty()
                || (state.my_tugs.len() < 2
                    && state.my_station.resources > 85.0
                    && (state.tick < 900
                        || state.my_rockets.len() >= 3
                        || state.my_station.resources > 140.0))
                || (state.tick < 2200
                    && state.my_tugs.len() < 3
                    && state.my_station.resources > 75.0
                    && (state.tick < 1000
                        || (!large_asteroids.is_empty() && state.my_rockets.len() >= 2)
                        || state.my_rockets.len() >= state.enemy_rockets.len()))
                || late_econ_tug;
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

        let mining_rocket_count = if severe_station_threat || large_asteroids.is_empty() {
            0
        } else if state.tick < 2400 {
            state
                .my_rockets
                .len()
                .min(if state.my_tugs.len() >= 2 { 2 } else { 1 })
        } else if state.my_station.resources < 90.0 && state.my_tugs.len() >= 2 {
            state.my_rockets.len().min(1)
        } else {
            0
        };
        let mut rockets_by_id: Vec<&RocketView> = state.my_rockets.iter().collect();
        rockets_by_id.sort_by_key(|r| r.id.0);
        let mut claimed_large = Vec::new();
        let tug_hunter_count = if !state.enemy_tugs.is_empty()
            && (state.enemy_tugs.len() > state.my_tugs.len()
                || state.enemy_station.resources > state.my_station.resources + 25.0)
            && rockets_by_id.len() > mining_rocket_count
        {
            1
        } else {
            0
        };

        for (rocket_idx, rocket) in rockets_by_id.iter().enumerate() {
            let rocket = *rocket;
            let nearest_threat = state.enemy_rockets.iter().min_by(|a, b| {
                state
                    .distance(rocket.position, a.position)
                    .partial_cmp(&state.distance(rocket.position, b.position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let defend_station = station_threat.is_some() && rocket.id.0 % 2 == 0;
            let mining_target = if rocket_idx < mining_rocket_count {
                large_asteroids
                    .iter()
                    .filter(|a| !claimed_large.contains(&a.id))
                    .min_by(|a, b| {
                        let rocket_bias_a = state.distance(rocket.position, a.position) * 0.9;
                        let rocket_bias_b = state.distance(rocket.position, b.position) * 0.9;
                        let station_bias_a = state.distance(state.my_station.position, a.position);
                        let station_bias_b = state.distance(state.my_station.position, b.position);
                        let size_bias_a = -(a.tier as f32) * 140.0;
                        let size_bias_b = -(b.tier as f32) * 140.0;
                        let score_a = rocket_bias_a + station_bias_a + size_bias_a;
                        let score_b = rocket_bias_b + station_bias_b + size_bias_b;
                        score_a
                            .partial_cmp(&score_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            } else {
                None
            };
            if let Some(ast) = mining_target {
                claimed_large.push(ast.id);
            }

            let hunt_tug = rocket_idx >= mining_rocket_count
                && rocket_idx < mining_rocket_count + tug_hunter_count;
            let tug_target = if hunt_tug {
                state.enemy_tugs.iter().min_by(|a, b| {
                    let carry_bias_a = if a.carrying.is_some() { -220.0 } else { 0.0 };
                    let carry_bias_b = if b.carrying.is_some() { -220.0 } else { 0.0 };
                    let score_a = state.distance(rocket.position, a.position)
                        + state.distance(a.position, state.enemy_station.position) * 0.25
                        + carry_bias_a;
                    let score_b = state.distance(rocket.position, b.position)
                        + state.distance(b.position, state.enemy_station.position) * 0.25
                        + carry_bias_b;
                    score_a
                        .partial_cmp(&score_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            } else {
                None
            };

            let (target_pos, target_vel, standoff) =
                if let Some(t) = station_threat.filter(|_| defend_station) {
                    (t.position, t.velocity, 180.0)
                } else if let Some(ast) = mining_target {
                    (ast.position, ast.velocity, ast.radius + 220.0)
                } else if let Some(tug) = tug_target {
                    (tug.position, tug.velocity, 170.0)
                } else if let Some(t) =
                    nearest_threat.filter(|t| state.distance(rocket.position, t.position) < 2200.0)
                {
                    (t.position, t.velocity, 220.0)
                } else {
                    let base = dv2(state, rocket.position, state.enemy_station.position);
                    let perp = Vec2::new(-base.y, base.x).normalize_or_zero();
                    let lane = (rocket.id.0 % 5) as f32 - 2.0;
                    let offset = perp * lane * 140.0;
                    (
                        [
                            state.enemy_station.position[0] + offset.x,
                            state.enemy_station.position[1] + offset.y,
                        ],
                        [0.0, 0.0],
                        390.0,
                    )
                };

            let cmd = fly_and_shoot(state, rocket, target_pos, target_vel, standoff);
            cmds.rockets.insert(rocket.id, cmd);
        }

        self.tug_targets.retain(|tid, aid| {
            state.my_tugs.iter().any(|t| t.id == *tid)
                && state.asteroids.iter().any(|a| {
                    a.id == *aid
                        && a.tier <= 2
                        && state.distance(a.position, state.my_station.position)
                            > beam_radius - 20.0
                })
        });
        let mut claimed: Vec<EntityId> = state.my_tugs.iter().filter_map(|t| t.carrying).collect();
        claimed.extend(self.tug_targets.values().copied());
        let incoming_station_bullets = state
            .bullets
            .iter()
            .filter(|b| {
                b.team != state.my_team
                    && state.distance(b.position, state.my_station.position) < beam_radius + 220.0
                    && approaching_point(
                        state,
                        b.position,
                        b.velocity_vec2(),
                        state.my_station.position,
                        100.0,
                    )
            })
            .count();
        let defense_tug_count = if state.my_tugs.len() >= 3
            && (severe_station_threat || incoming_station_bullets > 0)
        {
            1
        } else {
            0
        };
        let mut tugs_by_station: Vec<&TugView> = state.my_tugs.iter().collect();
        tugs_by_station.sort_by(|a, b| {
            state
                .distance(a.position, state.my_station.position)
                .partial_cmp(&state.distance(b.position, state.my_station.position))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let defender_tug_ids: Vec<EntityId> = tugs_by_station
            .into_iter()
            .take(defense_tug_count)
            .map(|t| t.id)
            .collect();

        for tug in &state.my_tugs {
            let mut cmd = TugCommand::default();
            let tug_vel = tug.velocity_vec2();
            let tug_health_ratio = tug.health / tug.max_health;
            let low_tug_count = state.my_tugs.len() <= 2;
            let nearby_enemy_rocket = state
                .enemy_rockets
                .iter()
                .any(|r| state.distance(tug.position, r.position) < 900.0);
            let should_repair_tug = tug_health_ratio < 0.45
                || (tug_health_ratio < 0.7 && (low_tug_count || nearby_enemy_rocket));
            let is_defender = defender_tug_ids.contains(&tug.id);
            if let Some(carried_id) = tug.carrying {
                if state
                    .asteroids
                    .iter()
                    .any(|a| a.id == carried_id && a.tier <= 2)
                {
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
                        cmd.beam_target = Some(carried_id);
                    }
                    cmds.tugs.insert(tug.id, cmd);
                    continue;
                }

                if should_repair_tug {
                    let to_station = dv2(state, tug.position, state.my_station.position);
                    let dist = to_station.length();
                    let dir = to_station.normalize_or_zero();
                    if dist < beam_radius - 24.0 {
                        cmd.beam_target = None;
                        let escape = -dir * 70.0;
                        cmd.thrust = [escape.x, escape.y];
                    } else {
                        let desired_v = dir * 145.0;
                        let dv = desired_v - tug_vel;
                        cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                        cmd.beam_target = Some(carried_id);
                    }
                    cmds.tugs.insert(tug.id, cmd);
                    continue;
                }

                if let Some(bullet) = state
                    .bullets
                    .iter()
                    .find(|b| b.id == carried_id && b.team != state.my_team)
                {
                    let away_station =
                        -dv2(state, bullet.position, state.my_station.position).normalize_or_zero();
                    let bullet_dir = bullet.velocity_vec2().normalize_or_zero();
                    let lateral = Vec2::new(-bullet_dir.y, bullet_dir.x).normalize_or_zero();
                    let side = if lateral.dot(away_station) >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    let desired_v = away_station * 110.0 + lateral * side * 80.0;
                    let dv = desired_v - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = Some(carried_id);
                    cmds.tugs.insert(tug.id, cmd);
                    continue;
                }

                if let Some(enemy) = state.enemy_rockets.iter().find(|r| r.id == carried_id) {
                    let away_station =
                        -dv2(state, enemy.position, state.my_station.position).normalize_or_zero();
                    let lateral = Vec2::new(-away_station.y, away_station.x).normalize_or_zero();
                    let side = if lateral.dot(dv2(state, enemy.position, tug.position)) >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    let desired_v = away_station * 120.0 + lateral * side * 60.0;
                    let dv = desired_v - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = Some(carried_id);
                    cmds.tugs.insert(tug.id, cmd);
                    continue;
                }

                cmd.beam_target = None;
                cmds.tugs.insert(tug.id, cmd);
                continue;
            }

            if should_repair_tug {
                let to_station = dv2(state, tug.position, state.my_station.position);
                let dist = to_station.length();
                let dir = to_station.normalize_or_zero();
                if dist < beam_radius - 18.0 {
                    let escape = -dir * 60.0;
                    cmd.thrust = [escape.x, escape.y];
                } else {
                    let desired_v = dir * 150.0;
                    let dv = desired_v - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                }
                cmds.tugs.insert(tug.id, cmd);
                continue;
            }

            if is_defender {
                if let Some(bullet) = state
                    .bullets
                    .iter()
                    .filter(|b| {
                        b.team != state.my_team
                            && state.distance(tug.position, b.position) < 170.0
                            && state.distance(b.position, state.my_station.position)
                                < beam_radius + 220.0
                            && approaching_point(
                                state,
                                b.position,
                                b.velocity_vec2(),
                                state.my_station.position,
                                100.0,
                            )
                    })
                    .min_by(|a, b| {
                        state
                            .distance(a.position, state.my_station.position)
                            .partial_cmp(&state.distance(b.position, state.my_station.position))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                {
                    let to_bullet = dv2(state, tug.position, bullet.position);
                    let desired_v =
                        to_bullet.normalize_or_zero() * 120.0 + bullet.velocity_vec2() * 0.15;
                    let dv = desired_v - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    cmd.beam_target = Some(bullet.id);
                    cmds.tugs.insert(tug.id, cmd);
                    continue;
                }

                if let Some(enemy) = state
                    .enemy_rockets
                    .iter()
                    .filter(|r| {
                        state.distance(tug.position, r.position) < 220.0
                            && state.distance(r.position, state.my_station.position) < 1600.0
                    })
                    .min_by(|a, b| {
                        state
                            .distance(a.position, state.my_station.position)
                            .partial_cmp(&state.distance(b.position, state.my_station.position))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                {
                    let to_enemy = dv2(state, tug.position, enemy.position);
                    let desired_v =
                        to_enemy.normalize_or_zero() * 110.0 + enemy.velocity_vec2() * 0.2;
                    let dv = desired_v - tug_vel;
                    cmd.thrust = [dv.x.clamp(-100.0, 100.0), dv.y.clamp(-100.0, 100.0)];
                    if to_enemy.length() < 170.0 {
                        cmd.beam_target = Some(enemy.id);
                    }
                    cmds.tugs.insert(tug.id, cmd);
                    continue;
                }
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
                        .filter(|a| {
                            a.tier <= 2
                                && !claimed.contains(&a.id)
                                && state.distance(a.position, state.my_station.position)
                                    > beam_radius
                        })
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
            let mut beamable_enemy_rockets: Vec<&RocketView> = state
                .enemy_rockets
                .iter()
                .filter(|r| {
                    state.distance(r.position, state.my_station.position) <= beam_radius
                        && approaching_point(
                            state,
                            r.position,
                            r.velocity_vec2(),
                            state.my_station.position,
                            25.0,
                        )
                })
                .collect();
            let urgent_station_pressure = severe_station_threat
                || incoming_station_bullets >= 2
                || state.my_station.health < state.my_station.max_health * 0.7
                || beamable_enemy_rockets.len() >= 2;
            if urgent_station_pressure {
                beamable_enemy_rockets.sort_by(|a, b| {
                    state
                        .distance(a.position, state.my_station.position)
                        .partial_cmp(&state.distance(b.position, state.my_station.position))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                for rocket in beamable_enemy_rockets {
                    if cmds.station.beam_targets.len() >= 5 {
                        break;
                    }
                    let away =
                        -dv2(state, rocket.position, state.my_station.position).normalize_or_zero();
                    let lateral = Vec2::new(-away.y, away.x).normalize_or_zero();
                    let side = if rocket.id.0 % 2 == 0 { 1.0 } else { -1.0 };
                    let force = (away * 0.8 + lateral * side * 0.2).normalize_or_zero();
                    cmds.station.beam_targets.push(BeamCommand {
                        target: rocket.id,
                        force_direction: [force.x, force.y],
                    });
                }
            }
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
            .my_tugs
            .iter()
            .filter(|t| {
                let ratio = t.health / t.max_health;
                ratio < 0.95
                    && (state.my_tugs.len() <= 2 || ratio < 0.75)
                    && state.distance(state.my_station.position, t.position) <= beam_radius
            })
            .map(|t| (t.id, t.health / t.max_health, t.position, 0_u8))
            .chain(
                state
                    .my_rockets
                    .iter()
                    .map(|r| (r.id, r.health / r.max_health, r.position, 1_u8)),
            )
            .filter(|(_, ratio, pos, _)| {
                *ratio < 0.9 && state.distance(state.my_station.position, *pos) <= beam_radius
            })
            .min_by(|a, b| {
                a.3.cmp(&b.3)
                    .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            });
        if let Some((id, _, _, _)) = repair_candidate {
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

fn approaching_point(
    state: &GameStateView,
    position: [f32; 2],
    velocity: Vec2,
    target: [f32; 2],
    threshold: f32,
) -> bool {
    let to_target = dv2(state, position, target).normalize_or_zero();
    velocity.dot(to_target) > threshold
}

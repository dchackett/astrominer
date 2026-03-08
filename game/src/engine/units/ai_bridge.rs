//! The AI bridge: extracts game state into GameStateView, calls player AIs,
//! and applies their commands back to the ECS.

use bevy::prelude::*;
use crate::api::{EntityId, GameStateView, StationView, RocketView, TugView, AsteroidView, BulletView, BuildProgressView, UnitTypeView};
use crate::config::GameConfig;
use crate::engine::physics::components::*;
use crate::engine::physics::collision_response::Bullet;
use crate::engine::physics::collision_detection::bounding_radius;
use crate::engine::asteroids::components::{Asteroid, AsteroidTier};
use super::components::*;

/// System that runs every FixedUpdate tick:
/// 1. Build GameStateView for each team
/// 2. Call each team's AI
/// 3. Store commands for application by other systems
pub fn run_player_ais(
    mut player_ais: ResMut<PlayerAIs>,
    tick: Res<TickCounter>,
    config: Res<GameConfig>,
    resources: Res<TeamResources>,
    stations: Query<(Entity, &Transform, &Team, &Health, &ShieldBubble, Option<&BuildProgress>, &BuildQueue), With<Station>>,
    rockets: Query<(Entity, &Transform, &Velocity, &Team, &Health, &ShootCooldown), With<Rocket>>,
    tugs: Query<(Entity, &Transform, &Velocity, &Team, &Health, Option<&CarryingAsteroid>), With<Tug>>,
    asteroids: Query<(Entity, &Transform, &Velocity, &AsteroidTier, &Health, &WireframeShape), With<Asteroid>>,
    bullets: Query<(Entity, &Transform, &Velocity, &Bullet)>,
) {
    // Build shared data (asteroids, bullets)
    let asteroid_views: Vec<AsteroidView> = asteroids.iter().map(|(e, tf, vel, tier, health, shape)| {
        AsteroidView {
            id: entity_to_id(e),
            position: [tf.translation.x, tf.translation.y],
            velocity: [vel.0.x, vel.0.y],
            tier: tier.0,
            health: health.current,
            max_health: health.max,
            radius: bounding_radius(shape),
        }
    }).collect();

    let bullet_views: Vec<BulletView> = bullets.iter().map(|(e, tf, vel, bullet)| {
        BulletView {
            id: entity_to_id(e),
            position: [tf.translation.x, tf.translation.y],
            velocity: [vel.0.x, vel.0.y],
            team: bullet.team.to_api(),
            remaining_lifetime: bullet.lifetime,
        }
    }).collect();

    // Build per-team views
    for team in [Team::Red, Team::Blue] {
        let api_team = team.to_api();

        let mut my_station = None;
        let mut enemy_station = None;
        for (e, tf, t, health, shield, bp, bq) in &stations {
            let sv = StationView {
                id: entity_to_id(e),
                position: [tf.translation.x, tf.translation.y],
                health: health.current,
                max_health: health.max,
                resources: resources.minerals(*t),
                beam_radius: shield.radius,
                build_progress: bp.map(|p| BuildProgressView {
                    unit_type: match p.unit_type {
                        UnitType::Rocket => UnitTypeView::Rocket,
                        UnitType::Tug => UnitTypeView::Tug,
                    },
                    progress: p.progress,
                    total: p.total,
                }),
                build_queue_length: bq.0.len(),
            };
            if *t == team {
                my_station = Some(sv);
            } else {
                enemy_station = Some(sv);
            }
        }

        let (my_rockets, enemy_rockets): (Vec<_>, Vec<_>) = rockets.iter()
            .map(|(e, tf, vel, t, health, cooldown)| {
                let rv = RocketView {
                    id: entity_to_id(e),
                    position: [tf.translation.x, tf.translation.y],
                    velocity: [vel.0.x, vel.0.y],
                    rotation: tf.rotation.to_euler(EulerRot::ZYX).0,
                    health: health.current,
                    max_health: health.max,
                    shoot_cooldown: cooldown.0,
                };
                (*t, rv)
            })
            .partition(|(t, _)| *t == team);

        let (my_tugs, enemy_tugs): (Vec<_>, Vec<_>) = tugs.iter()
            .map(|(e, tf, vel, t, health, carrying)| {
                let tv = TugView {
                    id: entity_to_id(e),
                    position: [tf.translation.x, tf.translation.y],
                    velocity: [vel.0.x, vel.0.y],
                    health: health.current,
                    max_health: health.max,
                    carrying: carrying.map(|c| entity_to_id(c.0)),
                };
                (*t, tv)
            })
            .partition(|(t, _)| *t == team);

        let state = GameStateView {
            tick: tick.0,
            my_team: api_team,
            world_width: config.world.width,
            world_height: config.world.height,
            my_station: my_station.unwrap_or_else(|| dead_station(team)),
            my_rockets: my_rockets.into_iter().map(|(_, r)| r).collect(),
            my_tugs: my_tugs.into_iter().map(|(_, t)| t).collect(),
            enemy_station: enemy_station.unwrap_or_else(|| dead_station(team)),
            enemy_rockets: enemy_rockets.into_iter().map(|(_, r)| r).collect(),
            enemy_tugs: enemy_tugs.into_iter().map(|(_, t)| t).collect(),
            asteroids: asteroid_views.clone(),
            bullets: bullet_views.clone(),
        };

        let commands = match team {
            Team::Red => player_ais.red.tick(&state),
            Team::Blue => player_ais.blue.tick(&state),
        };

        match team {
            Team::Red => player_ais.red_commands = commands,
            Team::Blue => player_ais.blue_commands = commands,
        }
    }

    // Increment tick counter (mutable borrow is fine since we already read it)
    // We do this via a separate system to avoid borrow issues
}

/// Increment the tick counter. Runs after AI.
pub fn increment_tick(mut tick: ResMut<TickCounter>) {
    tick.0 += 1;
}

/// Apply rocket commands from player AIs.
pub fn apply_rocket_commands(
    player_ais: Res<PlayerAIs>,
    mut rockets: Query<(Entity, &Team, &mut Thrust, &mut AngularVelocity), With<Rocket>>,
) {
    for (entity, team, mut thrust, mut ang_vel) in &mut rockets {
        let commands = player_ais.commands(*team);
        let id = entity_to_id(entity);
        if let Some(cmd) = commands.rockets.get(&id) {
            thrust.forward = cmd.thrust.clamp(0.0, 1.0) * thrust.max_forward;
            ang_vel.0 = cmd.rotation.clamp(-1.0, 1.0) * thrust.rotation_speed;
        } else {
            // No command = coast (no thrust, no rotation change)
            thrust.forward = 0.0;
            ang_vel.0 = 0.0;
        }
    }
}

/// Apply rocket shoot commands.
pub fn apply_rocket_shoot(
    mut commands: Commands,
    player_ais: Res<PlayerAIs>,
    mut rockets: Query<(Entity, &Transform, &Velocity, &Team, &mut ShootCooldown), With<Rocket>>,
    config: Res<GameConfig>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();

    for (entity, tf, vel, team, mut cooldown) in &mut rockets {
        cooldown.0 -= dt;
        if cooldown.0 > 0.0 { continue; }

        let cmds = player_ais.commands(*team);
        let id = entity_to_id(entity);
        if let Some(cmd) = cmds.rockets.get(&id) {
            if cmd.shoot {
                let forward = tf.local_y().truncate();
                let bullet_vel = forward * config.bullet.speed + vel.0;
                let spawn_pos = tf.translation.truncate() + forward * config.bullet.spawn_offset;
                let hs = config.bullet.half_size;

                commands.spawn((
                    Bullet { lifetime: config.bullet.lifetime, team: *team },
                    Velocity(bullet_vel),
                    AngularVelocity::default(),
                    Mass(config.bullet.mass),
                    WireframeShape {
                        vertices: vec![
                            Vec2::new(-hs, -hs),
                            Vec2::new(hs, -hs),
                            Vec2::new(hs, hs),
                            Vec2::new(-hs, hs),
                        ],
                        lines: vec![],
                    },
                    Transform::from_xyz(spawn_pos.x, spawn_pos.y, 0.5),
                    Visibility::default(),
                ));

                cooldown.0 = config.bullet.shoot_cooldown;
            }
        }
    }
}

/// Apply tug commands from player AIs.
pub fn apply_tug_commands(
    player_ais: Res<PlayerAIs>,
    mut tugs: Query<(Entity, &Team, &mut Velocity, &Thrust, Option<&CarryingAsteroid>), With<Tug>>,
    mut commands: Commands,
    _config: Res<GameConfig>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();

    for (entity, team, mut vel, thrust, carrying) in &mut tugs {
        let cmds = player_ais.commands(*team);
        let id = entity_to_id(entity);
        if let Some(cmd) = cmds.tugs.get(&id) {
            // Apply omnidirectional thrust
            let desired_thrust = Vec2::new(cmd.thrust[0], cmd.thrust[1]);
            let max = thrust.max_forward;
            let clamped = if desired_thrust.length() > max {
                desired_thrust.normalize() * max
            } else {
                desired_thrust
            };
            vel.0 += clamped * dt;

            // Handle beam target changes
            if let Some(target_id) = cmd.beam_target {
                // Player wants to beam a target - the tug_tractor_beam_force system
                // handles the actual physics. We just need to set CarryingAsteroid.
                if carrying.is_none() {
                    // Try to find the entity - we'll set it up as carrying
                    // The beam lock-on logic will validate range
                    commands.entity(entity).insert(CarryingAsteroid(id_to_entity(target_id)));
                }
            } else if carrying.is_some() {
                // Player wants to release
                commands.entity(entity).remove::<CarryingAsteroid>();
            }
        }
    }
}

/// Apply station build commands.
pub fn apply_station_commands(
    player_ais: Res<PlayerAIs>,
    mut stations: Query<(Entity, &Team, &mut BuildQueue), With<Station>>,
    _resources: Res<TeamResources>,
) {
    for (_entity, team, mut queue) in &mut stations {
        let cmds = player_ais.commands(*team);
        if let Some(build) = &cmds.station.build {
            let unit_type = match build {
                UnitTypeView::Rocket => UnitType::Rocket,
                UnitTypeView::Tug => UnitType::Tug,
            };
            queue.0.push(unit_type);
        }
    }
}

/// Apply station beam commands from player AIs.
/// When AI provides beam_targets, those override the auto-behavior for the targeted entities.
pub fn apply_station_beam_commands(
    player_ais: Res<PlayerAIs>,
    stations: Query<(&Transform, &ShieldBubble, &Team), With<Station>>,
    mut targets: Query<(&Transform, &mut Velocity, &Mass), Without<Station>>,
    bounds: Res<crate::engine::physics::WorldBounds>,
    config: Res<GameConfig>,
) {
    for (station_tf, shield, team) in &stations {
        let cmds = player_ais.commands(*team);
        if cmds.station.beam_targets.is_empty() { continue; }

        let station_pos = station_tf.translation.truncate();
        let max_beams = config.station.beam_count;

        for (i, beam_cmd) in cmds.station.beam_targets.iter().enumerate() {
            if i >= max_beams { break; }

            let target_entity = id_to_entity(beam_cmd.target);
            let Ok((target_tf, mut target_vel, target_mass)) = targets.get_mut(target_entity) else {
                continue;
            };

            let target_pos = target_tf.translation.truncate();
            let delta = bounds.shortest_delta(station_pos, target_pos);
            let dist = delta.length();

            // Only affect entities within beam radius
            if dist > shield.radius || dist < 0.1 { continue; }

            let force_dir = Vec2::new(beam_cmd.force_direction[0], beam_cmd.force_direction[1]);
            let force_dir = if force_dir.length() > 0.01 {
                force_dir.normalize()
            } else {
                // Default: pull toward station
                -delta.normalize()
            };

            let force = force_dir * config.station.pull_strength / target_mass.0;
            target_vel.0 += force * (1.0 / 60.0);
        }
    }
}

/// Apply station repair target command from player AIs.
/// When AI specifies a repair_target, that unit gets priority healing.
pub fn apply_station_repair_target(
    player_ais: Res<PlayerAIs>,
    stations: Query<(&Transform, &ShieldBubble, &Team), With<Station>>,
    mut units: Query<(&Transform, &mut Health, &Team), (Without<Station>, Without<Asteroid>)>,
    bounds: Res<crate::engine::physics::WorldBounds>,
    time: Res<Time<Fixed>>,
    config: Res<GameConfig>,
) {
    let dt = time.delta_secs();

    for (station_tf, shield, station_team) in &stations {
        let cmds = player_ais.commands(*station_team);
        let Some(target_id) = cmds.station.repair_target else { continue; };

        let station_pos = station_tf.translation.truncate();
        let target_entity = id_to_entity(target_id);

        let Ok((unit_tf, mut health, unit_team)) = units.get_mut(target_entity) else {
            continue;
        };

        // Must be same team
        if unit_team != station_team { continue; }
        // Must be in range
        let delta = bounds.shortest_delta(station_pos, unit_tf.translation.truncate());
        if delta.length() > shield.radius { continue; }
        // Must need healing
        if health.current >= health.max { continue; }

        // Boost repair rate for targeted unit (2x normal)
        let bonus = config.station.repair_rate * dt;
        health.current = (health.current + bonus).min(health.max);
    }
}

/// Tractor beam force physics (from original tug.rs).
pub fn tug_tractor_beam_force(
    mut tugs: Query<(Entity, &Transform, &CarryingAsteroid, &mut Velocity, &Mass), With<Tug>>,
    mut asteroids: Query<(&Transform, &mut Velocity, &Mass), (With<Asteroid>, Without<Tug>)>,
    bounds: Res<crate::engine::physics::WorldBounds>,
    mut commands: Commands,
    config: Res<GameConfig>,
) {
    for (tug_entity, tug_tf, carrying, mut tug_vel, tug_mass) in &mut tugs {
        let Ok((asteroid_tf, mut asteroid_vel, asteroid_mass)) = asteroids.get_mut(carrying.0) else {
            commands.entity(tug_entity).remove::<CarryingAsteroid>();
            continue;
        };

        let tug_pos = tug_tf.translation.truncate();
        let asteroid_pos = asteroid_tf.translation.truncate();
        let delta = bounds.shortest_delta(asteroid_pos, tug_pos);
        let dist = delta.length();

        if dist < 0.1 { continue; }

        if dist > config.tug.beam_break_range {
            commands.entity(tug_entity).remove::<CarryingAsteroid>();
            continue;
        }

        let dir = delta / dist;

        let displacement = dist - config.tug.beam_desired_distance;
        let force_mag = (displacement * config.tug.beam_strength / dist.max(1.0)).max(0.0);

        let rel_vel = asteroid_vel.0 - tug_vel.0;
        let rel_vel_along = rel_vel.dot(dir);
        let damp_force = -rel_vel_along * config.tug.beam_damping;

        let total_force = dir * (force_mag + damp_force);

        let asteroid_accel = total_force / asteroid_mass.0;
        let tug_accel = -total_force / tug_mass.0;

        let dt = 1.0 / 60.0;
        asteroid_vel.0 += asteroid_accel * dt;
        tug_vel.0 += tug_accel * dt;
    }
}

// ---- Helpers ----

fn entity_to_id(entity: Entity) -> EntityId {
    EntityId(entity.to_bits())
}

pub fn id_to_entity(id: EntityId) -> Entity {
    Entity::from_bits(id.0)
}

fn dead_station(_team: Team) -> StationView {
    StationView {
        id: EntityId(0),
        position: [0.0, 0.0],
        health: 0.0,
        max_health: 0.0,
        resources: 0.0,
        beam_radius: 0.0,
        build_progress: None,
        build_queue_length: 0,
    }
}

use bevy::prelude::*;
use std::f32::consts::TAU;
use crate::config::GameConfig;
use crate::engine::physics::components::*;
use crate::engine::physics::toroidal::WorldBounds;
use crate::engine::asteroids::components::{Asteroid, AsteroidTier};
use super::components::*;

/// Spawn both teams' stations.
pub fn spawn_stations(mut commands: Commands, config: Res<GameConfig>) {
    let n = config.station.num_sides;
    let radius = config.station.radius;
    let mut vertices = Vec::with_capacity(n);
    for i in 0..n {
        let angle = (i as f32 / n as f32) * TAU;
        vertices.push(Vec2::new(angle.cos() * radius, angle.sin() * radius));
    }

    // Red station: top half of map
    let red_y = config.world.height / 4.0;
    commands.spawn((
        Station,
        Team::Red,
        ShieldBubble { radius: config.station.beam_radius },
        BuildQueue(Vec::new()),
        StationBeamLocks { slots: vec![(0, 0.0); config.station.beam_count] },
        Health::new(config.station.health),
        Mass(config.station.mass),
        Velocity(Vec2::ZERO),
        AngularVelocity(0.0),
        WireframeShape { vertices: vertices.clone(), lines: vec![] },
        Transform::from_xyz(0.0, red_y, 0.0),
        Visibility::default(),
    ));

    // Blue station: bottom half of map
    let blue_y = -config.world.height / 4.0;
    commands.spawn((
        Station,
        Team::Blue,
        ShieldBubble { radius: config.station.beam_radius },
        BuildQueue(Vec::new()),
        StationBeamLocks { slots: vec![(0, 0.0); config.station.beam_count] },
        Health::new(config.station.health),
        Mass(config.station.mass),
        Velocity(Vec2::ZERO),
        AngularVelocity(0.0),
        WireframeShape { vertices, lines: vec![] },
        Transform::from_xyz(0.0, blue_y, 0.0),
        Visibility::default(),
    ));
}

/// Process build queue: tick progress, spawn units when complete.
pub fn station_build_tick(
    mut commands: Commands,
    mut stations: Query<(Entity, &Transform, &Team, &mut BuildQueue, Option<&mut BuildProgress>), With<Station>>,
    mut resources: ResMut<TeamResources>,
    time: Res<Time<Fixed>>,
    config: Res<GameConfig>,
) {
    let dt = time.delta_secs();

    for (entity, transform, team, mut queue, progress) in &mut stations {
        if let Some(mut bp) = progress {
            bp.progress += dt;
            if bp.progress >= bp.total {
                let pos = transform.translation.truncate();
                // Spawn away from station (toward enemy): Red spawns below, Blue spawns above
                let away_dir = match *team {
                    Team::Red => Vec2::new(0.0, -1.0),
                    Team::Blue => Vec2::new(0.0, 1.0),
                };
                let spawn_pos = pos + away_dir * config.station.spawn_offset;

                match bp.unit_type {
                    UnitType::Rocket => spawn_rocket(&mut commands, spawn_pos, *team, away_dir, &config),
                    UnitType::Tug => spawn_tug(&mut commands, spawn_pos, *team, &config),
                }

                commands.entity(entity).remove::<BuildProgress>();
            }
        } else if let Some(next) = queue.0.first().copied() {
            let cost = match next {
                UnitType::Rocket => config.economy.rocket_cost,
                UnitType::Tug => config.economy.tug_cost,
            };
            if resources.try_spend(*team, cost) {
                queue.0.remove(0);
                let build_time = match next {
                    UnitType::Rocket => config.economy.rocket_build_time,
                    UnitType::Tug => config.economy.tug_build_time,
                };
                commands.entity(entity).insert(BuildProgress {
                    unit_type: next,
                    progress: 0.0,
                    total: build_time,
                });
            }
        }
    }
}

pub fn spawn_rocket(commands: &mut Commands, pos: Vec2, team: Team, facing: Vec2, config: &GameConfig) {
    let vertices: Vec<Vec2> = config.rocket.vertices.iter()
        .map(|v| Vec2::new(v[0], v[1]))
        .collect();
    let lines = config.rocket.lines.clone();

    // Rocket's forward is +Y, so rotate to match the facing direction
    let angle = facing.y.atan2(facing.x) - std::f32::consts::FRAC_PI_2;

    commands.spawn((
        Rocket,
        team,
        Thrust {
            forward: 0.0,
            max_forward: config.rocket.max_thrust,
            rotation_speed: config.rocket.rotation_speed,
        },
        Health::new(config.rocket.health),
        Mass(config.rocket.mass),
        Velocity::default(),
        AngularVelocity::default(),
        ShootCooldown(0.0),
        WireframeShape { vertices, lines },
        Transform::from_xyz(pos.x, pos.y, 1.0)
            .with_rotation(Quat::from_rotation_z(angle)),
        Visibility::default(),
    ));
}

pub fn spawn_tug(commands: &mut Commands, pos: Vec2, team: Team, config: &GameConfig) {
    let hs = config.tug.half_size;
    let vertices = vec![
        Vec2::new(-hs, -hs),
        Vec2::new(hs, -hs),
        Vec2::new(hs, hs),
        Vec2::new(-hs, hs),
    ];

    commands.spawn((
        Tug,
        team,
        TractorBeam::default(),
        Health::new(config.tug.health),
        Mass(config.tug.mass),
        Velocity::default(),
        AngularVelocity::default(),
        Thrust {
            forward: 0.0,
            max_forward: config.tug.max_thrust,
            rotation_speed: config.tug.rotation_speed,
        },
        WireframeShape { vertices, lines: vec![] },
        Transform::from_xyz(pos.x, pos.y, 1.0),
        Visibility::default(),
    ));
}

/// Station tractor beam: repel large rocks, attract and slow small rocks for collection.
pub fn station_tractor_beam(
    stations: Query<(&Transform, &ShieldBubble), With<Station>>,
    mut asteroids: Query<(&Transform, &mut Velocity, &Mass, &AsteroidTier), (With<Asteroid>, Without<Station>)>,
    bounds: Res<WorldBounds>,
    config: Res<GameConfig>,
) {
    let beam_radius = config.station.beam_radius;

    for (station_tf, _shield) in &stations {
        let station_pos = station_tf.translation.truncate();

        for (asteroid_tf, mut vel, mass, tier) in &mut asteroids {
            let asteroid_pos = asteroid_tf.translation.truncate();
            let delta = bounds.shortest_delta(station_pos, asteroid_pos);
            let dist = delta.length();

            if dist > beam_radius || dist < 0.1 { continue; }

            let dir = delta / dist;

            if tier.0 <= config.asteroids.max_gatherable_tier {
                let force = -dir * config.station.pull_strength / mass.0;
                vel.0 += force * (1.0 / 60.0);
                let tangent = Vec2::new(-dir.y, dir.x);
                let tangential_vel = vel.0.dot(tangent);
                vel.0 -= tangent * tangential_vel * config.station.tangential_damping;
                let radial_vel = vel.0.dot(-dir);
                let target_speed = (dist * 0.15).min(config.station.max_inbound_speed);
                if radial_vel > target_speed {
                    vel.0 += dir * (radial_vel - target_speed) * config.station.radial_braking;
                }
            } else {
                // Repel large asteroids — use sqrt(mass) to keep force meaningful on heavy rocks
                let falloff = (beam_radius - dist) / beam_radius;
                let force = dir * config.station.repel_strength * falloff / mass.0.sqrt();
                vel.0 += force * (1.0 / 60.0);
            }
        }
    }
}

/// Repair nearby friendly units.
pub fn station_repair_nearby(
    stations: Query<(&Transform, &ShieldBubble, &Team), With<Station>>,
    mut units: Query<(&Transform, &mut Health, &Team), (Without<Station>, Without<Asteroid>)>,
    bounds: Res<WorldBounds>,
    time: Res<Time<Fixed>>,
    config: Res<GameConfig>,
) {
    let dt = time.delta_secs();

    for (station_tf, shield, station_team) in &stations {
        let station_pos = station_tf.translation.truncate();

        for (unit_tf, mut health, unit_team) in &mut units {
            if unit_team != station_team { continue; }
            let unit_pos = unit_tf.translation.truncate();
            let delta = bounds.shortest_delta(station_pos, unit_pos);
            if delta.length() < shield.radius {
                health.current = (health.current + config.station.repair_rate * dt).min(health.max);
            }
        }
    }
}

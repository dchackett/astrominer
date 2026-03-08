use bevy::prelude::*;
use std::f32::consts::TAU;
use crate::engine::units::components::*;
use crate::engine::physics::components::Velocity;
use crate::engine::physics::toroidal::WorldBounds;
use super::dust::spawn_dust_burst;

/// Draw station beam range as a dashed rotating circle.
pub fn render_station_beam_range(
    stations: Query<(&Transform, &Team), With<Station>>,
    mut gizmos: Gizmos,
    time: Res<Time>,
    config: Res<crate::config::GameConfig>,
) {
    let t = time.elapsed_secs();
    let beam_radius = config.station.beam_radius;

    for (transform, team) in &stations {
        let center = transform.translation.truncate();
        let segments = 64;
        let dash_on = 3;

        let color = match team {
            Team::Red => Color::srgba(1.0, 0.3, 0.3, 0.4),
            Team::Blue => Color::srgba(0.2, 0.5, 1.0, 0.4),
        };

        for i in 0..segments {
            if i % (dash_on + 1) >= dash_on {
                continue;
            }
            let a0 = (i as f32 / segments as f32) * TAU + t * 0.2;
            let a1 = ((i + 1) as f32 / segments as f32) * TAU + t * 0.2;
            let p0 = center + Vec2::new(a0.cos(), a0.sin()) * beam_radius;
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * beam_radius;
            gizmos.line_2d(p0, p1, color);
        }
    }
}

/// Draw tractor beam as perpendicular lines sliding from tug to asteroid.
pub fn render_tractor_beams(
    tugs: Query<(&Transform, &CarryingAsteroid, &Team), With<Tug>>,
    transforms: Query<&Transform>,
    bounds: Res<WorldBounds>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    let t = time.elapsed_secs();

    for (tug_tf, carrying, team) in &tugs {
        let Ok(asteroid_tf) = transforms.get(carrying.0) else { continue };
        let tug_pos = tug_tf.translation.truncate();
        let asteroid_pos = asteroid_tf.translation.truncate();
        let delta = bounds.shortest_delta(tug_pos, asteroid_pos);
        let beam_len = delta.length();
        if beam_len < 0.1 { continue; }

        let dir = delta / beam_len;
        let perp = Vec2::new(-dir.y, dir.x);

        let beam_color = match team {
            Team::Red => Color::srgba(1.0, 0.5, 0.4, 0.6),
            Team::Blue => Color::srgba(0.4, 0.7, 1.0, 0.6),
        };

        let num_rungs = 8;
        let rung_half_width = 10.0;
        for i in 0..num_rungs {
            let phase = ((t * 1.5 + i as f32 * 0.9) % 1.5) / 1.5;
            let along = phase * beam_len;
            let center = tug_pos + dir * along;
            let alpha = 1.0 - (phase - 0.5).abs() * 2.0;
            let c = beam_color.with_alpha(0.6 * alpha.max(0.1));
            gizmos.line_2d(center - perp * rung_half_width, center + perp * rung_half_width, c);
            let offset = dir * 1.0;
            gizmos.line_2d(center - perp * rung_half_width + offset, center + perp * rung_half_width + offset, c);
            gizmos.line_2d(center - perp * rung_half_width - offset, center + perp * rung_half_width - offset, c);
        }
    }
}

/// Draw station tractor beam effect on nearby asteroids.
pub fn render_station_beam(
    stations: Query<(&Transform, &Team), With<Station>>,
    asteroids: Query<(&Transform, &crate::engine::asteroids::components::AsteroidTier), With<crate::engine::asteroids::components::Asteroid>>,
    bounds: Res<WorldBounds>,
    mut gizmos: Gizmos,
    time: Res<Time>,
    config: Res<crate::config::GameConfig>,
) {
    let t = time.elapsed_secs();
    let beam_radius = config.station.beam_radius;

    for (station_tf, team) in &stations {
        let station_pos = station_tf.translation.truncate();

        for (asteroid_tf, tier) in &asteroids {
            let asteroid_pos = asteroid_tf.translation.truncate();
            let delta = bounds.shortest_delta(station_pos, asteroid_pos);
            let dist = delta.length();
            if dist > beam_radius || dist < 1.0 { continue; }

            let dir = delta / dist;
            let perp = Vec2::new(-dir.y, dir.x);

            if tier.0 <= config.asteroids.max_gatherable_tier {
                let num_rungs = 4;
                let rung_half_width = 6.0;
                let color = match team {
                    Team::Red => Color::srgba(1.0, 0.5, 0.4, 0.3),
                    Team::Blue => Color::srgba(0.4, 0.7, 1.0, 0.3),
                };
                for i in 0..num_rungs {
                    let phase = ((t * 1.0 + i as f32 * 0.8) % 1.2) / 1.2;
                    let along_pos = station_pos + dir * (dist * (1.0 - phase));
                    let alpha = 0.3 * (1.0 - (phase - 0.5).abs() * 2.0).max(0.1);
                    let c = color.with_alpha(alpha);
                    gizmos.line_2d(along_pos - perp * rung_half_width, along_pos + perp * rung_half_width, c);
                }
            } else {
                let num_rungs = 3;
                let rung_half_width = 8.0;
                for i in 0..num_rungs {
                    let phase = ((t * 1.5 + i as f32 * 0.7) % 1.2) / 1.2;
                    let along_pos = station_pos + dir * (dist * phase);
                    let alpha = 0.3 * (1.0 - (phase - 0.5).abs() * 2.0).max(0.1);
                    let c = Color::srgba(1.0, 0.4, 0.3, alpha);
                    gizmos.line_2d(along_pos - perp * rung_half_width, along_pos + perp * rung_half_width, c);
                }
            }
        }
    }
}

/// Spawn exhaust particles behind tugs when they're thrusting.
pub fn tug_exhaust_particles(
    mut commands: Commands,
    tugs: Query<(&Transform, &Velocity), With<Tug>>,
    mut prev_velocities: Local<std::collections::HashMap<Entity, Vec2>>,
    tug_entities: Query<Entity, With<Tug>>,
) {
    let mut current: std::collections::HashMap<Entity, (Vec2, Vec2)> = std::collections::HashMap::new();
    for (entity, (tf, vel)) in tug_entities.iter().zip(tugs.iter()) {
        current.insert(entity, (tf.translation.truncate(), vel.0));
    }

    for (&entity, &(pos, vel)) in &current {
        if let Some(&prev_vel) = prev_velocities.get(&entity) {
            let accel = vel - prev_vel;
            let accel_mag = accel.length();
            if accel_mag > 1.0 {
                let exhaust_dir = -accel.normalize_or_zero();
                let exhaust_pos = pos + exhaust_dir * 10.0;
                spawn_dust_burst(&mut commands, exhaust_pos, 2, accel_mag * 0.5, exhaust_dir * 30.0);
            }
        }
    }

    prev_velocities.clear();
    for (entity, &(_, vel)) in &current {
        prev_velocities.insert(*entity, vel);
    }
}

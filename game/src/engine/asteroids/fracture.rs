use bevy::prelude::*;
use rand::Rng;

use crate::config::GameConfig;
use crate::engine::game_state::rng::GameRng;
use crate::engine::physics::components::*;
use crate::engine::rendering::dust::spawn_dust_burst;
use super::components::*;
use super::generation::generate_asteroid_shape;

/// Queue of asteroids to fracture: (entity, tier).
#[derive(Resource, Default)]
pub struct FractureQueue(pub Vec<(Entity, u8)>);

/// Handle asteroid fracture: despawn parent, spawn children.
pub fn handle_fractures(
    mut commands: Commands,
    mut fracture_queue: ResMut<FractureQueue>,
    query: Query<(&Transform, &Velocity)>,
    mut rng: ResMut<GameRng>,
    config: Res<GameConfig>,
) {
    let pending: Vec<_> = fracture_queue.0.drain(..).collect();
    for (entity, tier) in pending {
        let Ok((transform, velocity)) = query.get(entity) else { continue };

        let pos = transform.translation.truncate();
        let parent_vel = velocity.0;

        // Spawn dust burst proportional to asteroid size
        let dust_count = match tier {
            6 => 40,
            5 => 30,
            4 => 20,
            3 => 14,
            2 => 10,
            1 => 8,
            _ => 4,
        };
        let dust_speed = 40.0 + tier as f32 * 15.0;
        spawn_dust_burst(&mut commands, pos, dust_count, dust_speed, parent_vel);

        commands.entity(entity).despawn();

        if tier <= 1 {
            continue;
        }

        let child_tier = tier - 1;
        let num_children = rng.0.gen_range(
            config.asteroids.fracture_children_min..=config.asteroids.fracture_children_max
        );

        for i in 0..num_children {
            let shape = generate_asteroid_shape(&mut rng.0, &config, child_tier);
            let child_radius = config.asteroids.tier(child_tier).radius;

            let angle = (i as f32 / num_children as f32) * std::f32::consts::TAU
                + rng.0.gen_range(-0.3..0.3);
            let offset_dist = child_radius * 1.5;
            let offset = Vec2::new(angle.cos() * offset_dist, angle.sin() * offset_dist);

            let spread_speed = config.asteroids.fracture_spread_speed;
            let spread = Vec2::new(angle.cos() * spread_speed, angle.sin() * spread_speed);
            let child_vel = parent_vel / num_children as f32 + spread;
            let spin = rng.0.gen_range(
                -config.asteroids.fracture_child_spin..config.asteroids.fracture_child_spin
            );

            let child_tier_comp = AsteroidTier(child_tier);
            let health_val = child_tier_comp.health(&config);
            let mass_val = child_tier_comp.mass(&config);

            commands.spawn((
                Asteroid,
                AsteroidTier(child_tier),
                Health::new(health_val),
                Mass(mass_val),
                Velocity(child_vel),
                AngularVelocity(spin),
                WireframeShape { vertices: shape, lines: vec![] },
                Transform::from_xyz(pos.x + offset.x, pos.y + offset.y, 0.0),
                Visibility::default(),
            ));
        }
    }
}

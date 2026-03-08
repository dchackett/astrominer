use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

use crate::config::GameConfig;
use crate::engine::game_state::rng::GameRng;
use crate::engine::physics::components::*;
use crate::engine::physics::toroidal::WorldBounds;
use crate::engine::physics::collision_response::Bullet;
use super::components::*;

/// Generate a random asteroid wireframe polygon.
pub fn generate_asteroid_shape(rng: &mut impl Rng, config: &GameConfig, tier: u8) -> Vec<Vec2> {
    let tier_config = config.asteroids.tier(tier);
    let base_radius = tier_config.radius;
    let n = tier_config.vertices;
    let mut vertices = Vec::with_capacity(n);
    for i in 0..n {
        let angle = (i as f32 / n as f32) * TAU
            + rng.gen_range(-config.asteroids.shape_angle_jitter..config.asteroids.shape_angle_jitter);
        let radius = base_radius
            * rng.gen_range(config.asteroids.shape_radius_min..config.asteroids.shape_radius_max);
        vertices.push(Vec2::new(angle.cos() * radius, angle.sin() * radius));
    }
    vertices
}

/// Spawn initial asteroid field.
pub fn spawn_asteroid_field(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    bounds: Res<WorldBounds>,
    config: Res<GameConfig>,
) {
    let hw = bounds.width / 2.0;
    let hh = bounds.height / 2.0;

    // Station positions for exclusion zones
    let red_station_y = config.world.height / 4.0;
    let blue_station_y = -config.world.height / 4.0;
    let station_positions = [
        Vec2::new(0.0, red_station_y),
        Vec2::new(0.0, blue_station_y),
    ];

    for (tier_idx, tier_config) in config.asteroids.tiers.iter().enumerate() {
        let tier_val = (tier_idx + 1) as u8;
        // Larger asteroids need bigger exclusion zones around stations
        let exclusion_radius = config.station.beam_radius + tier_config.radius * 2.0 + 200.0;

        for _ in 0..tier_config.spawn_count {
            let shape = generate_asteroid_shape(&mut rng.0, &config, tier_val);

            // Retry placement until we find a spot away from both stations
            let (x, y) = loop {
                let cx = rng.0.gen_range(-hw..hw);
                let cy = rng.0.gen_range(-hh..hh);
                let pos = Vec2::new(cx, cy);
                let too_close = station_positions.iter().any(|sp| {
                    bounds.shortest_delta(*sp, pos).length() < exclusion_radius
                });
                if !too_close { break (cx, cy); }
            };
            let speed_scale = tier_config.speed_scale;
            let max_speed = config.asteroids.max_initial_speed;
            let max_spin = config.asteroids.max_initial_spin;
            let vx = rng.0.gen_range(-max_speed..max_speed) * speed_scale;
            let vy = rng.0.gen_range(-max_speed..max_speed) * speed_scale;
            let spin = rng.0.gen_range(-max_spin..max_spin) * speed_scale;

            let tier_comp = AsteroidTier(tier_val);
            let health_val = tier_comp.health(&config);
            let mass_val = tier_comp.mass(&config);

            commands.spawn((
                Asteroid,
                AsteroidTier(tier_val),
                Health::new(health_val),
                Mass(mass_val),
                Velocity(Vec2::new(vx, vy)),
                AngularVelocity(spin),
                WireframeShape { vertices: shape, lines: vec![] },
                Transform::from_xyz(x, y, 0.0),
                Visibility::default(),
            ));
        }
    }
}

/// Tick bullet lifetimes and despawn expired ones.
pub fn tick_bullet_lifetime(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Bullet)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    for (entity, mut bullet) in &mut query {
        bullet.lifetime -= dt;
        if bullet.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

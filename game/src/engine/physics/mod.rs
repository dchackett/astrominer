use bevy::prelude::*;

pub mod collision_detection;
pub mod collision_response;
pub mod components;
pub mod integration;
pub mod toroidal;

pub use toroidal::WorldBounds;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<collision_detection::Collisions>()
            .add_systems(
                FixedUpdate,
                (
                    integration::apply_thrust,
                    integration::integrate_positions,
                    toroidal::wrap_positions,
                )
                    .chain()
                    .before(collision_detection::detect_collisions),
            )
            // Collision detection
            .add_systems(
                FixedUpdate,
                collision_detection::detect_collisions,
            )
            // Asteroid collisions
            .add_systems(
                FixedUpdate,
                (
                    collision_response::resolve_asteroid_asteroid,
                    collision_response::resolve_rocket_asteroid,
                    collision_response::resolve_tug_asteroid,
                )
                    .after(collision_detection::detect_collisions),
            )
            // Bullet collisions
            .add_systems(
                FixedUpdate,
                (
                    collision_response::resolve_bullet_asteroid,
                    collision_response::resolve_bullet_rocket,
                    collision_response::resolve_bullet_station,
                    collision_response::resolve_bullet_tug,
                    collision_response::resolve_bullet_bullet,
                )
                    .after(collision_detection::detect_collisions),
            )
            // Station collisions
            .add_systems(
                FixedUpdate,
                (
                    collision_response::resolve_station_asteroid,
                    collision_response::resolve_tug_station,
                    collision_response::resolve_rocket_station,
                )
                    .after(collision_detection::detect_collisions),
            )
            // Unit-unit collisions
            .add_systems(
                FixedUpdate,
                (
                    collision_response::resolve_rocket_rocket,
                    collision_response::resolve_rocket_tug,
                    collision_response::resolve_tug_tug,
                )
                    .after(collision_detection::detect_collisions),
            )
            // Dust effects
            .add_systems(
                FixedUpdate,
                collision_response::spawn_collision_dust
                    .after(collision_response::resolve_asteroid_asteroid),
            );
    }
}

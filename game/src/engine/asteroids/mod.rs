use bevy::prelude::*;

pub mod components;
pub mod fracture;
pub mod generation;

pub struct AsteroidPlugin;

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<fracture::FractureQueue>()
            .add_systems(Startup, generation::spawn_asteroid_field)
            .add_systems(
                FixedUpdate,
                (
                    fracture::handle_fractures,
                    generation::tick_bullet_lifetime,
                )
                    .after(crate::engine::physics::collision_response::resolve_bullet_asteroid),
            );
    }
}

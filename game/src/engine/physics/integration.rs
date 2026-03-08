use super::components::*;
use bevy::prelude::*;

/// Apply thrust force to velocity based on entity's facing direction.
pub fn apply_thrust(
    mut query: Query<(&Thrust, &Transform, &mut Velocity, &Mass)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    for (thrust, transform, mut vel, mass) in &mut query {
        if thrust.forward > 0.0 {
            let forward = transform.local_y().truncate();
            vel.0 += forward * (thrust.forward / mass.0) * dt;
        }
    }
}

/// Integrate positions from velocities (Euler integration).
pub fn integrate_positions(
    mut query: Query<(&Velocity, &AngularVelocity, &mut Transform)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    for (vel, ang_vel, mut transform) in &mut query {
        transform.translation.x += vel.0.x * dt;
        transform.translation.y += vel.0.y * dt;
        transform.rotate_z(ang_vel.0 * dt);
        transform.rotation = transform.rotation.normalize();
    }
}

use bevy::prelude::*;

/// A single dust particle: a little dot that flies outward and fades.
#[derive(Component)]
pub struct DustParticle {
    pub velocity: Vec2,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

/// Spawn a burst of dust particles at a position.
pub fn spawn_dust_burst(commands: &mut Commands, pos: Vec2, count: usize, speed: f32, base_vel: Vec2) {
    use std::f32::consts::TAU;
    for i in 0..count {
        let angle = (i as f32 / count as f32) * TAU + (i as f32 * 1.618) % 1.0 * 0.5;
        let spread = Vec2::new(angle.cos(), angle.sin()) * speed * (0.5 + (i as f32 * 0.7) % 1.0);
        let vel = base_vel * 0.3 + spread;
        let lifetime = 0.4 + (i as f32 * 0.3) % 0.4;

        commands.spawn((
            DustParticle {
                velocity: vel,
                lifetime,
                max_lifetime: lifetime,
            },
            Transform::from_xyz(pos.x, pos.y, 0.5),
            Visibility::default(),
        ));
    }
}

/// Update dust particles: move them and despawn when expired.
pub fn tick_dust_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DustParticle, &mut Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, mut dust, mut transform) in &mut query {
        dust.lifetime -= dt;
        if dust.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.x += dust.velocity.x * dt;
        transform.translation.y += dust.velocity.y * dt;
        dust.velocity *= 0.97;
    }
}

/// Render dust particles as small dots/lines.
pub fn render_dust_particles(
    query: Query<(&DustParticle, &Transform)>,
    mut gizmos: Gizmos,
) {
    for (dust, transform) in &query {
        let alpha = (dust.lifetime / dust.max_lifetime).clamp(0.0, 1.0);
        let pos = transform.translation.truncate();
        let end = pos + dust.velocity.normalize_or_zero() * 2.0;
        let color = Color::srgba(0.8, 0.8, 0.8, alpha);
        gizmos.line_2d(pos, end, color);
    }
}

use bevy::prelude::*;

#[derive(Resource)]
pub struct WorldBounds {
    pub width: f32,
    pub height: f32,
}

impl WorldBounds {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Wrap a position to stay within bounds. World is centered at origin.
    pub fn wrap(&self, pos: Vec2) -> Vec2 {
        let hw = self.width / 2.0;
        let hh = self.height / 2.0;
        Vec2::new(
            wrap_coord(pos.x, -hw, hw),
            wrap_coord(pos.y, -hh, hh),
        )
    }

    /// Shortest displacement from `from` to `to` on the torus.
    pub fn shortest_delta(&self, from: Vec2, to: Vec2) -> Vec2 {
        let mut dx = to.x - from.x;
        let mut dy = to.y - from.y;
        if dx > self.width / 2.0 { dx -= self.width; }
        if dx < -self.width / 2.0 { dx += self.width; }
        if dy > self.height / 2.0 { dy -= self.height; }
        if dy < -self.height / 2.0 { dy += self.height; }
        Vec2::new(dx, dy)
    }
}

fn wrap_coord(val: f32, min: f32, max: f32) -> f32 {
    let range = max - min;
    if val < min {
        val + range
    } else if val >= max {
        val - range
    } else {
        val
    }
}

/// Wrap entity positions to stay within world bounds.
pub fn wrap_positions(
    bounds: Res<WorldBounds>,
    mut query: Query<&mut Transform>,
) {
    for mut transform in &mut query {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let wrapped = bounds.wrap(pos);
        transform.translation.x = wrapped.x;
        transform.translation.y = wrapped.y;
    }
}

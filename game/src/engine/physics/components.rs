use bevy::prelude::*;

#[derive(Component, Default, Clone)]
pub struct Velocity(pub Vec2);

#[derive(Component, Default)]
pub struct AngularVelocity(pub f32);

#[derive(Component)]
pub struct Mass(pub f32);

#[derive(Component, Clone)]
pub struct WireframeShape {
    pub vertices: Vec<Vec2>,
    /// Line segments as pairs of vertex indices. If empty, draws as a closed polygon.
    pub lines: Vec<[usize; 2]>,
}

#[derive(Component)]
pub struct Thrust {
    pub forward: f32,
    pub max_forward: f32,
    pub rotation_speed: f32,
}

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

impl Default for Mass {
    fn default() -> Self {
        Self(1.0)
    }
}

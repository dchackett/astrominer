use bevy::math::Vec2;
use serde::{Deserialize, Serialize};

/// Unique identifier for entities within a game tick.
/// These are stable across a single tick but may be recycled across ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Which team a player is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Team {
    Red,
    Blue,
}

/// Complete game state snapshot visible to a player AI each tick.
/// No fog of war — both teams see everything.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateView {
    /// Current game tick (0-indexed, increments at 60Hz)
    pub tick: u64,
    /// Which team this view is for
    pub my_team: Team,
    /// World dimensions
    pub world_width: f32,
    pub world_height: f32,

    // My units
    pub my_station: StationView,
    pub my_rockets: Vec<RocketView>,
    pub my_tugs: Vec<TugView>,

    // Enemy units
    pub enemy_station: StationView,
    pub enemy_rockets: Vec<RocketView>,
    pub enemy_tugs: Vec<TugView>,

    // Shared objects
    pub asteroids: Vec<AsteroidView>,
    pub bullets: Vec<BulletView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationView {
    pub id: EntityId,
    pub position: [f32; 2],
    pub health: f32,
    pub max_health: f32,
    pub resources: f32,
    pub beam_radius: f32,
    /// What's currently being built, if anything
    pub build_progress: Option<BuildProgressView>,
    pub build_queue_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildProgressView {
    pub unit_type: UnitTypeView,
    pub progress: f32,
    pub total: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitTypeView {
    Rocket,
    Tug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketView {
    pub id: EntityId,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub rotation: f32,
    pub health: f32,
    pub max_health: f32,
    pub shoot_cooldown: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TugView {
    pub id: EntityId,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub health: f32,
    pub max_health: f32,
    /// Entity ID of the asteroid being towed, if any
    pub carrying: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsteroidView {
    pub id: EntityId,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub tier: u8,
    pub health: f32,
    pub max_health: f32,
    pub radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletView {
    pub id: EntityId,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub team: Team,
    pub remaining_lifetime: f32,
}

// Helper methods for the state views

impl GameStateView {
    /// Compute shortest delta from `from` to `to` on the torus.
    pub fn shortest_delta(&self, from: [f32; 2], to: [f32; 2]) -> [f32; 2] {
        let mut dx = to[0] - from[0];
        let mut dy = to[1] - from[1];
        let hw = self.world_width / 2.0;
        let hh = self.world_height / 2.0;
        if dx > hw { dx -= self.world_width; }
        if dx < -hw { dx += self.world_width; }
        if dy > hh { dy -= self.world_height; }
        if dy < -hh { dy += self.world_height; }
        [dx, dy]
    }

    /// Toroidal distance between two points.
    pub fn distance(&self, from: [f32; 2], to: [f32; 2]) -> f32 {
        let d = self.shortest_delta(from, to);
        (d[0] * d[0] + d[1] * d[1]).sqrt()
    }
}

impl StationView {
    pub fn position_vec2(&self) -> Vec2 {
        Vec2::new(self.position[0], self.position[1])
    }
}

impl RocketView {
    pub fn position_vec2(&self) -> Vec2 {
        Vec2::new(self.position[0], self.position[1])
    }
    pub fn velocity_vec2(&self) -> Vec2 {
        Vec2::new(self.velocity[0], self.velocity[1])
    }
    /// Unit vector in the direction the rocket is facing (local Y axis).
    pub fn forward(&self) -> Vec2 {
        Vec2::new(-self.rotation.sin(), self.rotation.cos())
    }
}

impl TugView {
    pub fn position_vec2(&self) -> Vec2 {
        Vec2::new(self.position[0], self.position[1])
    }
    pub fn velocity_vec2(&self) -> Vec2 {
        Vec2::new(self.velocity[0], self.velocity[1])
    }
}

impl AsteroidView {
    pub fn position_vec2(&self) -> Vec2 {
        Vec2::new(self.position[0], self.position[1])
    }
    pub fn velocity_vec2(&self) -> Vec2 {
        Vec2::new(self.velocity[0], self.velocity[1])
    }
}

impl BulletView {
    pub fn position_vec2(&self) -> Vec2 {
        Vec2::new(self.position[0], self.position[1])
    }
    pub fn velocity_vec2(&self) -> Vec2 {
        Vec2::new(self.velocity[0], self.velocity[1])
    }
}

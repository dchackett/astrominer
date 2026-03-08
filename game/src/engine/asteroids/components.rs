use bevy::prelude::*;
use crate::config::GameConfig;

#[derive(Component)]
pub struct Asteroid;

#[derive(Component)]
pub struct AsteroidTier(pub u8);

impl AsteroidTier {
    pub fn base_radius(&self, config: &GameConfig) -> f32 {
        config.asteroids.tier(self.0).radius
    }

    pub fn num_vertices(&self, config: &GameConfig) -> usize {
        config.asteroids.tier(self.0).vertices
    }

    pub fn health(&self, config: &GameConfig) -> f32 {
        config.asteroids.tier(self.0).health
    }

    pub fn mass(&self, config: &GameConfig) -> f32 {
        config.asteroids.mass_for_tier(self.0)
    }
}

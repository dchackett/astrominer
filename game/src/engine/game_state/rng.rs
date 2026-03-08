use bevy::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[derive(Resource)]
pub struct GameRng(pub ChaCha8Rng);

impl GameRng {
    pub fn new(seed: u64) -> Self {
        Self(ChaCha8Rng::seed_from_u64(seed))
    }
}

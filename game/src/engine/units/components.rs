use bevy::prelude::*;
use crate::api;

/// Marker for rockets.
#[derive(Component)]
pub struct Rocket;

/// Marker for tugs.
#[derive(Component)]
pub struct Tug;

/// Marker for stations.
#[derive(Component)]
pub struct Station;

/// Team membership.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Team {
    Red,
    Blue,
}

impl Team {
    pub fn to_api(&self) -> api::Team {
        match self {
            Team::Red => api::Team::Red,
            Team::Blue => api::Team::Blue,
        }
    }
}

/// Station shield/beam radius (visual + gameplay).
#[derive(Component)]
pub struct ShieldBubble {
    pub radius: f32,
}

/// Tractor beam component for tugs.
#[derive(Component)]
pub struct TractorBeam {
    pub target: Option<Entity>,
    pub strength: f32,
    pub range: f32,
}

impl Default for TractorBeam {
    fn default() -> Self {
        Self {
            target: None,
            strength: 80.0,
            range: 150.0,
        }
    }
}

/// Tug is carrying this asteroid via tractor beam.
#[derive(Component)]
pub struct CarryingAsteroid(pub Entity);

#[derive(Clone, Copy, PartialEq)]
pub enum UnitType {
    Rocket,
    Tug,
}

/// Build queue on a station.
#[derive(Component)]
pub struct BuildQueue(pub Vec<UnitType>);

/// Current build in progress.
#[derive(Component)]
pub struct BuildProgress {
    pub unit_type: UnitType,
    pub progress: f32,
    pub total: f32,
}

/// Per-team resources, stored as a single resource to allow both teams' stations to access it.
#[derive(Resource)]
pub struct TeamResources {
    pub red_minerals: f32,
    pub blue_minerals: f32,
}

impl TeamResources {
    pub fn new(starting: f32) -> Self {
        Self {
            red_minerals: starting,
            blue_minerals: starting,
        }
    }

    pub fn minerals(&self, team: Team) -> f32 {
        match team {
            Team::Red => self.red_minerals,
            Team::Blue => self.blue_minerals,
        }
    }

    pub fn minerals_mut(&mut self, team: Team) -> &mut f32 {
        match team {
            Team::Red => &mut self.red_minerals,
            Team::Blue => &mut self.blue_minerals,
        }
    }

    pub fn add(&mut self, team: Team, amount: f32) {
        *self.minerals_mut(team) += amount;
    }

    pub fn try_spend(&mut self, team: Team, amount: f32) -> bool {
        let m = self.minerals_mut(team);
        if *m >= amount {
            *m -= amount;
            true
        } else {
            false
        }
    }
}

/// Shoot cooldown timer for rockets.
#[derive(Component)]
pub struct ShootCooldown(pub f32);

/// Game tick counter.
#[derive(Resource, Default)]
pub struct TickCounter(pub u64);

/// Holds boxed player AIs and their pending commands.
#[derive(Resource)]
pub struct PlayerAIs {
    pub red: Box<dyn api::PlayerAI>,
    pub blue: Box<dyn api::PlayerAI>,
    pub red_commands: api::Commands,
    pub blue_commands: api::Commands,
}

impl PlayerAIs {
    pub fn new(red: Box<dyn api::PlayerAI>, blue: Box<dyn api::PlayerAI>) -> Self {
        Self {
            red,
            blue,
            red_commands: api::Commands::default(),
            blue_commands: api::Commands::default(),
        }
    }

    pub fn commands(&self, team: Team) -> &api::Commands {
        match team {
            Team::Red => &self.red_commands,
            Team::Blue => &self.blue_commands,
        }
    }
}

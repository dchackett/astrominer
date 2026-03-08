use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::state::{EntityId, UnitTypeView};

/// All commands issued by a player AI for a single tick.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Commands {
    /// Commands for each rocket, keyed by entity ID
    pub rockets: HashMap<EntityId, RocketCommand>,
    /// Commands for each tug, keyed by entity ID
    pub tugs: HashMap<EntityId, TugCommand>,
    /// Command for the station
    pub station: StationCommand,
}

/// Command for a single rocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocketCommand {
    /// Thrust fraction: 0.0 (no thrust) to 1.0 (full thrust). Clamped.
    pub thrust: f32,
    /// Rotation fraction: -1.0 (full CCW) to 1.0 (full CW). Clamped.
    pub rotation: f32,
    /// Whether to fire a bullet this tick (subject to cooldown).
    pub shoot: bool,
}

impl Default for RocketCommand {
    fn default() -> Self {
        Self {
            thrust: 0.0,
            rotation: 0.0,
            shoot: false,
        }
    }
}

/// Command for a single tug.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TugCommand {
    /// Desired thrust direction and magnitude.
    /// The vector is clamped to max thrust. [0,0] means no thrust.
    pub thrust: [f32; 2],
    /// Entity ID of the asteroid to tractor beam. None to release/idle beam.
    pub beam_target: Option<EntityId>,
}

impl Default for TugCommand {
    fn default() -> Self {
        Self {
            thrust: [0.0, 0.0],
            beam_target: None,
        }
    }
}

/// Command for the station.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StationCommand {
    /// Queue a unit to build. None means don't queue anything this tick.
    pub build: Option<UnitTypeView>,
    /// Tractor beam targets. Up to beam_count targets.
    /// Each entry is (target_entity_id, force_direction).
    /// Force direction is a unit vector indicating which way to push/pull the target.
    /// The magnitude is handled by the engine based on station beam strength.
    pub beam_targets: Vec<BeamCommand>,
    /// Entity ID of a friendly unit to repair. Must be within beam radius.
    pub repair_target: Option<EntityId>,
}

/// A single tractor beam command from the station.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeamCommand {
    /// Which entity to target
    pub target: EntityId,
    /// Direction to apply force (unit vector, will be normalized by engine).
    /// Positive = toward station, negative = away. Or specify arbitrary direction.
    pub force_direction: [f32; 2],
}

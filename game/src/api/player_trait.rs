use crate::config::GameConfig;
use super::state::{GameStateView, Team};
use super::commands::Commands;

/// Result of a completed game.
#[derive(Debug, Clone)]
pub struct GameResult {
    pub winner: Option<Team>,
    pub reason: String,
    pub ticks_played: u64,
    pub red_station_health: f32,
    pub blue_station_health: f32,
}

/// The trait that all player AIs must implement.
///
/// Players own their struct and can store any persistent state they want
/// between ticks as fields on their struct. The engine calls `&mut self`
/// so you have full mutable access to your own state each tick.
///
/// # Example
///
/// ```rust
/// struct MyAI {
///     target_assignments: HashMap<EntityId, EntityId>,
///     ticks_since_last_build: u64,
/// }
///
/// impl PlayerAI for MyAI {
///     fn tick(&mut self, state: &GameStateView) -> Commands {
///         // Use self.target_assignments, update self.ticks_since_last_build, etc.
///         Commands::default()
///     }
/// }
/// ```
pub trait PlayerAI: Send + Sync {
    /// Called once at game start with the config and team assignment.
    /// Override to initialize your state based on game parameters.
    fn init(&mut self, _config: &GameConfig, _team: Team) {}

    /// Called every tick (60Hz). Receives complete game state, returns commands.
    /// This is where all your logic lives.
    fn tick(&mut self, state: &GameStateView) -> Commands;

    /// Called when the game ends. Override to log results, update strategies, etc.
    fn on_game_over(&mut self, _result: &GameResult) {}

    /// A name for this AI, used in logs and match results.
    fn name(&self) -> &str {
        "Unnamed AI"
    }
}

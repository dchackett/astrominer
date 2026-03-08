use crate::api::{PlayerAI, GameStateView, Commands};

/// A do-nothing AI that issues no commands. Useful as a starting point.
pub struct DoNothingAI;

impl PlayerAI for DoNothingAI {
    fn tick(&mut self, _state: &GameStateView) -> Commands {
        Commands::default()
    }

    fn name(&self) -> &str {
        "DoNothing"
    }
}

//! Runner module: game logging, headless execution, and match orchestration.

pub mod game_log;

use bevy::prelude::*;
use game_log::GameLog;

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        // Snapshot every 300 ticks (5 seconds of game time)
        app.insert_resource(GameLog::new(300))
            .add_systems(FixedUpdate, game_log::detect_events
                .after(crate::engine::units::game_rules::check_win_condition))
            .add_systems(FixedUpdate, game_log::write_game_log
                .after(game_log::detect_events));
    }
}

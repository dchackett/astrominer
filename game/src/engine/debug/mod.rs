use bevy::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
    }
}

use bevy::prelude::*;

pub mod rng;

#[derive(Resource, Default)]
pub struct Paused(pub bool);

/// Whether running in headless mode (no rendering, no input).
#[derive(Resource)]
pub struct HeadlessMode(pub bool);

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Paused>();

        let headless = app.world().get_resource::<HeadlessMode>()
            .map(|h| h.0)
            .unwrap_or(false);

        if !headless {
            app.add_systems(Update, toggle_pause);
        }
    }
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut paused: ResMut<Paused>,
    mut time: ResMut<Time<Virtual>>,
) {
    if keyboard.just_pressed(KeyCode::KeyP) {
        paused.0 = !paused.0;
        if paused.0 {
            time.pause();
        } else {
            time.unpause();
        }
    }
}

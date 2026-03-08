use bevy::prelude::*;

mod api;
mod config;
mod engine;
mod players;
mod runner;

use api::PlayerAI;
use config::GameConfig;
use engine::units::components::{PlayerAIs, TeamResources};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let headless = args.iter().any(|a| a == "--headless");

    // Load config from file or use defaults
    let config = GameConfig::load_or_default("config.toml");

    // Create player AIs
    let mut red_ai = players::example_ai::ExampleAI::new();
    let mut blue_ai = players::example_ai::ExampleAI::new();
    red_ai.init(&config, api::Team::Red);
    blue_ai.init(&config, api::Team::Blue);

    let player_ais = PlayerAIs::new(
        Box::new(red_ai),
        Box::new(blue_ai),
    );

    let mut app = App::new();

    if headless {
        // Headless mode: minimal plugins, no window, no rendering
        app.add_plugins(MinimalPlugins);
    } else {
        // Graphical mode: full rendering
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AstroMiner".into(),
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }));
    }

    // Insert config and resources before plugins run
    app.insert_resource(engine::game_state::HeadlessMode(headless))
        .insert_resource(config.clone())
        .insert_resource(engine::game_state::rng::GameRng::new(config.world.rng_seed))
        .insert_resource(Time::<Fixed>::from_hz(config.world.tick_rate_hz))
        .insert_resource(engine::physics::WorldBounds::new(config.world.width, config.world.height))
        .insert_resource(TeamResources::new(config.economy.starting_minerals))
        .insert_resource(player_ais)
        // Core game plugins (always needed)
        .add_plugins(engine::game_state::GameStatePlugin)
        .add_plugins(engine::physics::PhysicsPlugin)
        .add_plugins(engine::asteroids::AsteroidPlugin)
        .add_plugins(engine::units::UnitsPlugin)
        // Logging (always enabled)
        .add_plugins(runner::LoggingPlugin);

    if headless {
        // Speed up time and exit when game is over
        app.add_systems(Update, headless_speed_up)
            .add_systems(FixedUpdate, exit_on_game_over
                .after(runner::game_log::write_game_log));
    } else {
        // Graphical-only plugins
        app.add_plugins(engine::rendering::RenderingPlugin)
            .add_plugins(engine::debug::DebugPlugin);
    }

    app.run();
}

/// In headless mode, speed up virtual time so fixed timestep runs many ticks per frame.
fn headless_speed_up(mut time: ResMut<Time<Virtual>>) {
    time.set_relative_speed(1000.0);
    time.set_max_delta(std::time::Duration::from_secs(60));
}

/// In headless mode, exit the app once the game is over and log is written.
fn exit_on_game_over(
    game_over: Res<engine::units::game_rules::GameOverState>,
) {
    if game_over.is_over {
        std::process::exit(0);
    }
}

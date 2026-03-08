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

    // Parse --red <name> and --blue <name> args
    let red_name = get_arg(&args, "--red").unwrap_or_else(|| "example".to_string());
    let blue_name = get_arg(&args, "--blue").unwrap_or_else(|| "example".to_string());

    // Load config from file or use defaults
    let config = GameConfig::load_or_default("config.toml");

    // Create player AIs
    let mut red_ai = create_ai(&red_name);
    let mut blue_ai = create_ai(&blue_name);
    red_ai.init(&config, api::Team::Red);
    blue_ai.init(&config, api::Team::Blue);

    println!("Red: {} vs Blue: {}", red_ai.name(), blue_ai.name());

    let player_ais = PlayerAIs::new(red_ai, blue_ai);

    let mut app = App::new();

    if headless {
        app.add_plugins(MinimalPlugins);
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("AstroMiner — {} vs {}", red_name, blue_name).into(),
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
        app.add_systems(Update, headless_speed_up)
            .add_systems(FixedUpdate, exit_on_game_over
                .after(runner::game_log::write_game_log));
    } else {
        app.add_plugins(engine::rendering::RenderingPlugin)
            .add_plugins(engine::debug::DebugPlugin);
    }

    app.run();
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn create_ai(name: &str) -> Box<dyn PlayerAI> {
    match name {
        "example" => Box::new(players::example_ai::ExampleAI::new()),
        "aggressive_miner" => Box::new(players::aggressive_miner::AggressiveMinerAI::new()),
        "do_nothing" => Box::new(players::do_nothing::DoNothingAI),
        other => {
            eprintln!("Unknown AI: '{}'. Available: example, aggressive_miner, do_nothing", other);
            std::process::exit(1);
        }
    }
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

use bevy::prelude::*;

pub mod ai_bridge;
pub mod components;
pub mod game_rules;
pub mod station;

use components::*;

pub struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TickCounter>()
            .init_resource::<game_rules::GameOverState>()
            .init_resource::<ActiveStationBeams>()
            .add_systems(Startup, station::spawn_stations)
            // AI commands
            .add_systems(
                FixedUpdate,
                (
                    ai_bridge::run_player_ais,
                    ai_bridge::apply_rocket_commands,
                    ai_bridge::apply_tug_commands,
                    ai_bridge::apply_station_commands,
                    ai_bridge::apply_rocket_shoot,
                )
                    .chain(),
            )
            // Station AI beam/repair commands
            .add_systems(
                FixedUpdate,
                (
                    ai_bridge::apply_station_beam_commands,
                    ai_bridge::apply_station_repair_target,
                )
                    .after(ai_bridge::apply_rocket_shoot),
            )
            // Engine mechanics
            .add_systems(
                FixedUpdate,
                (
                    station::station_build_tick,
                    station::station_tractor_beam,
                    station::station_repair_nearby,
                    ai_bridge::tug_tractor_beam_force,
                )
                    .chain()
                    .after(ai_bridge::apply_station_beam_commands),
            )
            // Game rules (run after collision resolution)
            .add_systems(
                FixedUpdate,
                (
                    game_rules::despawn_dead_units,
                    game_rules::station_self_repair,
                    game_rules::check_win_condition,
                    ai_bridge::increment_tick,
                )
                    .chain()
                    .after(station::station_repair_nearby),
            );
    }
}

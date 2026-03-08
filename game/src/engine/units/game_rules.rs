//! Game rules: unit death, station self-repair, win condition.

use bevy::prelude::*;
use crate::api::{GameResult, Team as ApiTeam};
use crate::config::GameConfig;
use crate::engine::physics::components::*;
use crate::engine::rendering::dust::spawn_dust_burst;
use crate::engine::asteroids::components::Asteroid;
use super::components::*;

/// Despawn any non-asteroid entity whose health has dropped to zero or below.
/// Spawns a dust burst at death location.
pub fn despawn_dead_units(
    mut commands: Commands,
    query: Query<(Entity, &Transform, &Health), (Without<Asteroid>, Without<Station>)>,
) {
    for (entity, tf, health) in &query {
        if health.current <= 0.0 {
            let pos = tf.translation.truncate();
            spawn_dust_burst(&mut commands, pos, 15, 60.0, Vec2::ZERO);
            commands.entity(entity).despawn();
        }
    }
}

/// Game over state.
#[derive(Resource, Default)]
pub struct GameOverState {
    pub is_over: bool,
    pub result: Option<GameResult>,
}

/// Check if either station is destroyed. If so, trigger game over.
pub fn check_win_condition(
    stations: Query<(&Health, &Team), With<Station>>,
    tick: Res<TickCounter>,
    mut game_over: ResMut<GameOverState>,
    mut player_ais: ResMut<PlayerAIs>,
    time: Option<ResMut<Time<Virtual>>>,
) {
    if game_over.is_over { return; }

    let mut red_health = 0.0f32;
    let mut blue_health = 0.0f32;
    let mut red_alive = false;
    let mut blue_alive = false;

    for (health, team) in &stations {
        match team {
            Team::Red => {
                red_health = health.current;
                red_alive = health.current > 0.0;
            }
            Team::Blue => {
                blue_health = health.current;
                blue_alive = health.current > 0.0;
            }
        }
    }

    // Check if a station entity is missing entirely (despawned)
    // or has health <= 0
    let red_dead = !red_alive;
    let blue_dead = !blue_alive;

    if !red_dead && !blue_dead { return; }

    let (winner, reason) = if red_dead && blue_dead {
        (None, "Both stations destroyed simultaneously".to_string())
    } else if red_dead {
        (Some(ApiTeam::Blue), "Red station destroyed".to_string())
    } else {
        (Some(ApiTeam::Red), "Blue station destroyed".to_string())
    };

    let result = GameResult {
        winner,
        reason,
        ticks_played: tick.0,
        red_station_health: red_health,
        blue_station_health: blue_health,
    };

    // Notify AIs
    player_ais.red.on_game_over(&result);
    player_ais.blue.on_game_over(&result);

    // Print result
    println!("=== GAME OVER ===");
    println!("Winner: {:?}", result.winner);
    println!("Reason: {}", result.reason);
    println!("Ticks: {}", result.ticks_played);
    println!("Red HP: {:.0}, Blue HP: {:.0}", result.red_station_health, result.blue_station_health);

    game_over.is_over = true;
    game_over.result = Some(result);

    // Pause the game (only in graphical mode)
    if let Some(mut time) = time {
        time.pause();
    }
}

/// Station self-repair: spend resources to heal station over time.
pub fn station_self_repair(
    mut stations: Query<(&mut Health, &Team), With<Station>>,
    mut resources: ResMut<TeamResources>,
    time: Res<Time<Fixed>>,
    config: Res<GameConfig>,
) {
    let dt = time.delta_secs();

    for (mut health, team) in &mut stations {
        if health.current >= health.max { continue; }

        let heal_amount = config.station.self_repair_rate * dt;
        let cost = heal_amount * config.station.repair_cost_per_hp;

        if resources.minerals(*team) >= cost {
            *resources.minerals_mut(*team) -= cost;
            health.current = (health.current + heal_amount).min(health.max);
        }
    }
}

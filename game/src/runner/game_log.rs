//! Compact game logging for agent-driven AI iteration.
//! Produces a single JSON file per game with summary, events, and periodic snapshots.

use bevy::prelude::*;
use serde::Serialize;

use crate::engine::physics::components::*;
use crate::engine::physics::collision_response::Bullet;
use crate::engine::asteroids::components::Asteroid;
use crate::engine::units::components::*;
use crate::engine::units::game_rules::GameOverState;

/// Resource that accumulates game events and periodic snapshots.
#[derive(Resource)]
pub struct GameLog {
    pub events: Vec<GameEvent>,
    pub snapshots: Vec<Snapshot>,
    pub snapshot_interval: u64,
    /// Track unit counts per team for summary
    pub red_units_built: u32,
    pub blue_units_built: u32,
    pub red_units_lost: u32,
    pub blue_units_lost: u32,
    pub red_minerals_mined: f32,
    pub blue_minerals_mined: f32,
    /// Previous tick's resource values for detecting income
    prev_red_minerals: f32,
    prev_blue_minerals: f32,
    /// Previous tick's unit counts for detecting spawns/deaths
    prev_red_rockets: usize,
    prev_blue_rockets: usize,
    prev_red_tugs: usize,
    prev_blue_tugs: usize,
    /// Track asteroid count for detecting mining
    prev_asteroid_count: usize,
    written: bool,
}

impl GameLog {
    pub fn new(snapshot_interval: u64) -> Self {
        Self {
            events: Vec::new(),
            snapshots: Vec::new(),
            snapshot_interval,
            red_units_built: 0,
            blue_units_built: 0,
            red_units_lost: 0,
            blue_units_lost: 0,
            red_minerals_mined: 0.0,
            blue_minerals_mined: 0.0,
            prev_red_minerals: 0.0,
            prev_blue_minerals: 0.0,
            prev_red_rockets: 0,
            prev_blue_rockets: 0,
            prev_red_tugs: 0,
            prev_blue_tugs: 0,
            prev_asteroid_count: 0,
            written: false,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct GameEvent {
    pub tick: u64,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Snapshot {
    pub tick: u64,
    pub red: TeamSnapshot,
    pub blue: TeamSnapshot,
    pub asteroids_remaining: usize,
    pub bullets_in_flight: usize,
}

#[derive(Serialize, Clone)]
pub struct TeamSnapshot {
    pub station_health: f32,
    pub minerals: f32,
    pub rockets: usize,
    pub tugs: usize,
}

#[derive(Serialize)]
pub struct GameLogOutput {
    pub result: GameResultJson,
    pub summary: GameSummary,
    pub events: Vec<GameEvent>,
    pub snapshots: Vec<Snapshot>,
}

#[derive(Serialize)]
pub struct GameResultJson {
    pub winner: Option<String>,
    pub reason: String,
    pub ticks_played: u64,
    pub game_time_secs: f32,
    pub red_station_health: f32,
    pub blue_station_health: f32,
}

#[derive(Serialize)]
pub struct GameSummary {
    pub red_units_built: u32,
    pub blue_units_built: u32,
    pub red_units_lost: u32,
    pub blue_units_lost: u32,
    pub red_minerals_mined: f32,
    pub blue_minerals_mined: f32,
}

/// System: detect game events by diffing state each tick.
pub fn detect_events(
    tick: Res<TickCounter>,
    mut log: ResMut<GameLog>,
    resources: Res<TeamResources>,
    rockets: Query<&Team, With<Rocket>>,
    tugs: Query<&Team, With<Tug>>,
    stations: Query<(&Health, &Team), With<Station>>,
    asteroids: Query<(), With<Asteroid>>,
    bullets: Query<(), With<Bullet>>,
) {
    let t = tick.0;

    // Count current units
    let mut red_rockets = 0usize;
    let mut blue_rockets = 0usize;
    let mut red_tugs = 0usize;
    let mut blue_tugs = 0usize;
    for team in &rockets {
        match team { Team::Red => red_rockets += 1, Team::Blue => blue_rockets += 1 }
    }
    for team in &tugs {
        match team { Team::Red => red_tugs += 1, Team::Blue => blue_tugs += 1 }
    }
    let asteroid_count = asteroids.iter().count();

    // Skip tick 0 (initialization)
    if t > 0 {
        // Detect unit spawns (count increased)
        if red_rockets > log.prev_red_rockets {
            log.red_units_built += (red_rockets - log.prev_red_rockets) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_spawned".into(),
                team: Some("Red".into()), detail: Some("Rocket".into()),
            });
        }
        if blue_rockets > log.prev_blue_rockets {
            log.blue_units_built += (blue_rockets - log.prev_blue_rockets) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_spawned".into(),
                team: Some("Blue".into()), detail: Some("Rocket".into()),
            });
        }
        if red_tugs > log.prev_red_tugs {
            log.red_units_built += (red_tugs - log.prev_red_tugs) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_spawned".into(),
                team: Some("Red".into()), detail: Some("Tug".into()),
            });
        }
        if blue_tugs > log.prev_blue_tugs {
            log.blue_units_built += (blue_tugs - log.prev_blue_tugs) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_spawned".into(),
                team: Some("Blue".into()), detail: Some("Tug".into()),
            });
        }

        // Detect unit losses (count decreased)
        if red_rockets < log.prev_red_rockets {
            log.red_units_lost += (log.prev_red_rockets - red_rockets) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_destroyed".into(),
                team: Some("Red".into()), detail: Some("Rocket".into()),
            });
        }
        if blue_rockets < log.prev_blue_rockets {
            log.blue_units_lost += (log.prev_blue_rockets - blue_rockets) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_destroyed".into(),
                team: Some("Blue".into()), detail: Some("Rocket".into()),
            });
        }
        if red_tugs < log.prev_red_tugs {
            log.red_units_lost += (log.prev_red_tugs - red_tugs) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_destroyed".into(),
                team: Some("Red".into()), detail: Some("Tug".into()),
            });
        }
        if blue_tugs < log.prev_blue_tugs {
            log.blue_units_lost += (log.prev_blue_tugs - blue_tugs) as u32;
            log.events.push(GameEvent {
                tick: t, event: "unit_destroyed".into(),
                team: Some("Blue".into()), detail: Some("Tug".into()),
            });
        }

        // Track mineral income (resources went up = mined something)
        let red_m = resources.minerals(Team::Red);
        let blue_m = resources.minerals(Team::Blue);
        if red_m > log.prev_red_minerals {
            log.red_minerals_mined += red_m - log.prev_red_minerals;
        }
        if blue_m > log.prev_blue_minerals {
            log.blue_minerals_mined += blue_m - log.prev_blue_minerals;
        }
    }

    // Update previous state
    log.prev_red_rockets = red_rockets;
    log.prev_blue_rockets = blue_rockets;
    log.prev_red_tugs = red_tugs;
    log.prev_blue_tugs = blue_tugs;
    log.prev_red_minerals = resources.minerals(Team::Red);
    log.prev_blue_minerals = resources.minerals(Team::Blue);
    log.prev_asteroid_count = asteroid_count;

    // Periodic snapshot
    if t % log.snapshot_interval == 0 {
        let mut red_hp = 0.0f32;
        let mut blue_hp = 0.0f32;
        for (health, team) in &stations {
            match team {
                Team::Red => red_hp = health.current,
                Team::Blue => blue_hp = health.current,
            }
        }

        log.snapshots.push(Snapshot {
            tick: t,
            red: TeamSnapshot {
                station_health: red_hp,
                minerals: resources.minerals(Team::Red),
                rockets: red_rockets,
                tugs: red_tugs,
            },
            blue: TeamSnapshot {
                station_health: blue_hp,
                minerals: resources.minerals(Team::Blue),
                rockets: blue_rockets,
                tugs: blue_tugs,
            },
            asteroids_remaining: asteroid_count,
            bullets_in_flight: bullets.iter().count(),
        });
    }
}

/// System: write game log to file when game ends.
pub fn write_game_log(
    game_over: Res<GameOverState>,
    mut log: ResMut<GameLog>,
) {
    if !game_over.is_over || log.written { return; }
    log.written = true;

    let Some(ref result) = game_over.result else { return };

    let output = GameLogOutput {
        result: GameResultJson {
            winner: result.winner.map(|w| format!("{:?}", w)),
            reason: result.reason.clone(),
            ticks_played: result.ticks_played,
            game_time_secs: result.ticks_played as f32 / 60.0,
            red_station_health: result.red_station_health,
            blue_station_health: result.blue_station_health,
        },
        summary: GameSummary {
            red_units_built: log.red_units_built,
            blue_units_built: log.blue_units_built,
            red_units_lost: log.red_units_lost,
            blue_units_lost: log.blue_units_lost,
            red_minerals_mined: log.red_minerals_mined,
            blue_minerals_mined: log.blue_minerals_mined,
        },
        events: log.events.clone(),
        snapshots: log.snapshots.clone(),
    };

    // Write to a single file (overwritten each game, not accumulated)
    match serde_json::to_string_pretty(&output) {
        Ok(json) => {
            if let Err(e) = std::fs::write("game_log.json", &json) {
                eprintln!("Failed to write game log: {e}");
            } else {
                println!("Game log written to game_log.json ({} events, {} snapshots)",
                    output.events.len(), output.snapshots.len());
            }
        }
        Err(e) => eprintln!("Failed to serialize game log: {e}"),
    }
}

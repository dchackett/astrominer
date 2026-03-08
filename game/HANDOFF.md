# AstroMiner Handoff

## What is this?

AstroMiner is a programming game. You write a Rust AI that controls a team of units (rockets, tugs, station) competing against another AI in a real-time asteroid mining and combat game. Read `DESIGN.md` for full game rules.

## Quick Start

### Run a game (graphical)
```bash
cargo run
```
Controls: WASD to pan, scroll to zoom, P to pause. Works after game over too.

### Run a game (headless, ~5 seconds)
```bash
cargo run -- --headless
```
Outputs `game_log.json` with result, summary, events, and periodic snapshots.

### Read the game log
```bash
cat game_log.json | head -20
```

## Writing an AI

### 1. Create your AI file

Create a new file in `src/players/`, e.g. `src/players/my_ai.rs`:

```rust
use std::collections::HashMap;
use bevy::math::Vec2;
use crate::api::*;
use crate::config::GameConfig;

pub struct MyAI {
    team: Option<Team>,
    // Add any persistent state you need here
}

impl MyAI {
    pub fn new() -> Self {
        Self { team: None }
    }
}

impl PlayerAI for MyAI {
    fn name(&self) -> &str { "MyAI" }

    fn init(&mut self, config: &GameConfig, team: Team) {
        self.team = Some(team);
    }

    fn tick(&mut self, state: &GameStateView) -> Commands {
        let mut cmds = Commands::default();

        // Control each rocket
        for rocket in &state.my_rockets {
            let mut cmd = RocketCommand::default();
            cmd.thrust = 1.0;    // Full thrust forward
            cmd.rotation = 0.0;  // No turn
            cmd.shoot = false;   // Don't shoot
            cmds.rockets.insert(rocket.id, cmd);
        }

        // Control each tug
        for tug in &state.my_tugs {
            let mut cmd = TugCommand::default();
            cmd.thrust = [0.0, 0.0];     // No thrust
            cmd.beam_target = None;       // No beam
            cmds.tugs.insert(tug.id, cmd);
        }

        // Station: queue a rocket build
        if state.my_station.resources >= 100.0
            && state.my_station.build_progress.is_none()
            && state.my_station.build_queue_length == 0
        {
            cmds.station.build = Some(UnitTypeView::Rocket);
        }

        cmds
    }
}
```

### 2. Register your AI

In `src/players/mod.rs`, add:
```rust
pub mod my_ai;
```

In `src/main.rs`, swap in your AI:
```rust
let mut red_ai = players::my_ai::MyAI::new();
// or
let mut blue_ai = players::my_ai::MyAI::new();
```

### 3. Run and iterate
```bash
cargo run -- --headless   # Fast run, check game_log.json
cargo run                 # Watch it play visually
```

## API Reference

### GameStateView (what you see each tick)

```rust
state.tick           // Current tick number (u64, 60 per second)
state.my_team        // Team::Red or Team::Blue
state.world_width    // 10000.0
state.world_height   // 10000.0

state.my_station     // StationView
state.my_rockets     // Vec<RocketView>
state.my_tugs        // Vec<TugView>

state.enemy_station  // StationView
state.enemy_rockets  // Vec<RocketView>
state.enemy_tugs     // Vec<TugView>

state.asteroids      // Vec<AsteroidView>
state.bullets        // Vec<BulletView>
```

### Entity Views

```rust
// StationView
station.id                  // EntityId
station.position            // [f32; 2]
station.health              // f32
station.max_health           // f32
station.resources            // f32 (your mineral count)
station.beam_radius          // f32 (320.0)
station.build_progress       // Option<BuildProgressView>
station.build_queue_length   // usize

// RocketView
rocket.id                   // EntityId
rocket.position             // [f32; 2]
rocket.velocity             // [f32; 2]
rocket.rotation             // f32 (radians, 0 = up)
rocket.health               // f32
rocket.max_health            // f32
rocket.shoot_cooldown        // f32 (seconds until can shoot, <= 0 means ready)

// Helper methods:
rocket.velocity_vec2()      // -> Vec2
rocket.forward()            // -> Vec2 (unit vector rocket is facing)

// TugView
tug.id                      // EntityId
tug.position                // [f32; 2]
tug.velocity                // [f32; 2]
tug.health                  // f32
tug.max_health               // f32
tug.carrying                 // Option<EntityId> (asteroid being towed)

// AsteroidView
asteroid.id                 // EntityId
asteroid.position           // [f32; 2]
asteroid.velocity           // [f32; 2]
asteroid.tier               // u8 (1-6, 1-2 are gatherable)
asteroid.health             // f32
asteroid.max_health          // f32
asteroid.radius              // f32

// BulletView
bullet.id                   // EntityId
bullet.position             // [f32; 2]
bullet.velocity             // [f32; 2]
bullet.team                  // Team
bullet.remaining_lifetime    // f32
```

### Toroidal Distance (IMPORTANT)

The world wraps. Never subtract positions directly. Always use:
```rust
let delta = state.shortest_delta(from_pos, to_pos);  // -> [f32; 2]
let dist = state.distance(from_pos, to_pos);          // -> f32
```

### Commands (what you send each tick)

```rust
let mut cmds = Commands::default();

// Rocket: thrust (0-1), rotation (-1 to 1), shoot (bool)
cmds.rockets.insert(rocket.id, RocketCommand {
    thrust: 1.0,
    rotation: 0.5,
    shoot: true,
});

// Tug: 2D thrust vector, beam target
cmds.tugs.insert(tug.id, TugCommand {
    thrust: [50.0, -30.0],  // Omnidirectional, clamped to max
    beam_target: Some(asteroid_id),
});

// Station: build, beam targets, repair
cmds.station.build = Some(UnitTypeView::Rocket);
cmds.station.beam_targets = vec![BeamCommand {
    target: entity_id,
    force_direction: [0.0, -1.0],
}];
cmds.station.repair_target = Some(damaged_unit_id);
```

### Persistent State

Your AI struct persists across ticks via `&mut self`. Store whatever you want:
```rust
pub struct MyAI {
    target_assignments: HashMap<EntityId, EntityId>,
    phase: GamePhase,
    last_mineral_count: f32,
    // etc.
}
```

No filesystem I/O needed. Your state lives as long as the game runs.

## Project Structure

```
game/
  Cargo.toml
  config.toml              # All gameplay constants (editable)
  DESIGN.md                # Full game rules
  HANDOFF.md               # This file
  game_log.json            # Output from last game run
  src/
    main.rs                # Entry point, app setup
    config.rs              # GameConfig struct (mirrors config.toml)
    api/                   # Player-facing interface
      mod.rs               # Re-exports everything
      state.rs             # GameStateView and all *View types
      commands.rs          # Commands, RocketCommand, TugCommand, StationCommand
      player_trait.rs      # PlayerAI trait, GameResult
    players/               # AI implementations (EDIT THESE)
      mod.rs
      example_ai.rs        # Competitive reference AI
      do_nothing.rs        # Minimal stub
    engine/                # Game engine (DO NOT EDIT for fair play)
      physics/             # Newtonian physics, collisions
      asteroids/           # Asteroid spawning, fracture
      units/               # Unit lifecycle, AI bridge, game rules
      rendering/           # Wireframes, HUD, camera (graphical only)
      game_state/          # Pause, RNG
      debug/               # Frame time diagnostics
    runner/                # Game logging
      game_log.rs          # JSON log output
```

## Game Log Format

`game_log.json` is overwritten each game. Structure:

```json
{
  "result": {
    "winner": "Red",
    "reason": "Blue station destroyed",
    "ticks_played": 8274,
    "game_time_secs": 137.9,
    "red_station_health": 500.0,
    "blue_station_health": 0.0
  },
  "summary": {
    "red_units_built": 6,
    "blue_units_built": 6,
    "red_units_lost": 0,
    "blue_units_lost": 1,
    "red_minerals_mined": 425.0,
    "blue_minerals_mined": 425.0
  },
  "events": [
    {"tick": 241, "event": "unit_spawned", "team": "Red", "detail": "Tug"},
    {"tick": 6561, "event": "unit_destroyed", "team": "Blue", "detail": "Tug"}
  ],
  "snapshots": [
    {
      "tick": 0,
      "red": {"station_health": 500.0, "minerals": 125.0, "rockets": 0, "tugs": 0},
      "blue": {"station_health": 500.0, "minerals": 125.0, "rockets": 0, "tugs": 0},
      "asteroids_remaining": 129,
      "bullets_in_flight": 0
    }
  ]
}
```

Snapshots are taken every 300 ticks (5 game seconds).

## Tips for AI Development

1. **Use toroidal distance** for everything. `state.distance()` and `state.shortest_delta()` handle wrapping.
2. **Friendly fire is on.** Check what's in your line of fire before shooting.
3. **Rockets have Newtonian physics.** No friction. Plan your velocity changes.
4. **`rocket.forward()`** gives the direction the rocket is facing. Thrust applies force in this direction.
5. **Tugs have omnidirectional thrust.** They don't need to face their movement direction.
6. **Beam lock range is 112 units** for tugs. Get close to grab an asteroid.
7. **Station auto-gathers** tier 1-2 asteroids within 320 units. Just get them close enough.
8. **Build early.** Starting minerals (200) can buy 2 tugs (150) with change left over.
9. **Mine large asteroids** by shooting them to fracture into gatherable sizes.
10. **The station is immobile.** It can't dodge. Rockets at standoff range shooting consistently will kill it.

## Tech Details

- **Rust + Bevy 0.18.1** game engine
- **60 Hz fixed timestep**, deterministic with seeded RNG
- **Entity IDs**: `EntityId(u64)` — stable within a tick, may be recycled across ticks after entity death
- **Config**: All constants in `config.toml` / `GameConfig`. Available to AIs via `init()`.
- **No network**: Both AIs run in the same process. `&mut self` on the trait gives persistent state.

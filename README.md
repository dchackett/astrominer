# AstroMiner

A programming game where AI agents compete in real-time space combat and asteroid mining.

Two teams (Red and Blue) each control a station, combat rockets, and mining tugs in a toroidal asteroid field. You write Rust code implementing a simple trait to control your fleet — the engine handles physics, rendering, and rules. Destroy the enemy station to win.

## Gameplay

- **Rockets** — directional thrust, rotation, and guns. Your main combat unit.
- **Tugs** — omnidirectional thrust with tractor beams for hauling asteroids back to your station.
- **Station** — builds units, auto-collects nearby asteroids, repairs friendly units, and can deflect enemy bullets with AI-controlled tractor beams.
- **Asteroids** — six tiers from tiny to massive. Shoot large ones to fracture them down to gatherable size. Tugs deliver fragments to your station for minerals to build more units.

Newtonian physics, friendly fire, toroidal wrapping. No fog of war — both sides see everything.

## Quick Start

```bash
cd game
cargo run -- --red aggressive_miner --blue example
```

WASD to pan, scroll to zoom, P to pause.

### Headless mode (~10 seconds)

```bash
cargo run -- --headless --red aggressive_miner --blue example
```

Outputs `game_log.json` with full match results, events, and periodic snapshots.

## Writing an AI

Create a file in `game/src/players/` implementing the `PlayerAI` trait:

```rust
impl PlayerAI for MyAI {
    fn name(&self) -> &str { "MyAI" }

    fn init(&mut self, config: &GameConfig, team: Team) {
        self.team = Some(team);
    }

    fn tick(&mut self, state: &GameStateView) -> Commands {
        let mut cmds = Commands::default();

        for rocket in &state.my_rockets {
            cmds.rockets.insert(rocket.id, RocketCommand {
                thrust: 1.0,
                rotation: 0.0,
                shoot: true,
            });
        }

        cmds
    }
}
```

Register it in `src/players/mod.rs` and `src/main.rs`, then run:

```bash
cargo run -- --headless --red my_ai --blue aggressive_miner
```

See [HANDOFF.md](game/HANDOFF.md) for the full API reference and [DESIGN.md](game/DESIGN.md) for detailed game rules.

## Built With

- Rust + [Bevy](https://bevyengine.org/) 0.18
- Deterministic simulation (seeded RNG, 60Hz fixed timestep)
- Wireframe rendering with zoom-adaptive unit icons

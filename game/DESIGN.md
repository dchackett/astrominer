# AstroMiner Game Design

AstroMiner is a programming game. Two AI teams (Red and Blue) compete in a real-time strategy game set in an asteroid field. Players write Rust code implementing the `PlayerAI` trait to control all their units. The game engine handles physics, rendering, and rules.

## World

- **Toroidal space**: 20,000 x 20,000 units, wrapping on all edges
- **Deterministic**: Seeded RNG (ChaCha8), fixed 60Hz timestep
- **No fog of war**: Both teams see the complete game state every tick

## Units

### Station
Each team has one station. Destroying the enemy station wins the game.

| Stat | Value |
|------|-------|
| Health | 1000 HP |
| Mass | 1000 |
| Position | Red at (0, +5000), Blue at (0, -5000) |
| Beam radius | 320 units |
| Beam count | 5 simultaneous beams |
| Beam acquire time | 0.1 seconds per new target |

**Behaviors:**
- **Tractor beam (auto)**: Automatically pulls small asteroids (tier 1-2) toward itself for mineral collection, repels large asteroids
- **Tractor beam (AI-controlled)**: AI can direct up to `beam_count` beams at specific targets with force directions. Beams require a short acquisition delay (0.1s) when locking onto a new target, but maintain lock as long as the target stays in range. Can be used to deflect enemy bullets.
- **Build queue**: AI can queue Rockets or Tugs for construction (costs minerals, takes time)
- **Repair**: Automatically heals friendly units within beam radius at 5 HP/s
- **Targeted repair**: AI can designate a specific unit for bonus healing (additional 5 HP/s)
- **Self-repair**: Spends 2 minerals/s to heal itself at 2 HP/s when damaged

### Rocket
Combat unit. Moves with directional thrust, rotates, and shoots bullets.

| Stat | Value |
|------|-------|
| Health | 100 HP |
| Mass | 5 |
| Max thrust | 250 |
| Rotation speed | 4 rad/s |
| Cost | 50 minerals |
| Build time | 5 seconds |

**Controls** (per rocket, per tick):
- `thrust`: 0.0 to 1.0 — forward engine power
- `rotation`: -1.0 to 1.0 — turning rate (CCW to CW)
- `shoot`: bool — fire a bullet (subject to 0.2s cooldown)

**Physics**: Thrust applies force in the direction the rocket is facing (local Y axis). Rotation changes heading. No friction — Newtonian physics, objects keep their velocity. Rockets spawn facing away from their station.

### Tug
Utility unit. Omnidirectional thrust, tractor beam for hauling asteroids.

| Stat | Value |
|------|-------|
| Health | 60 HP |
| Mass | 10 |
| Max thrust | 100 |
| Cost | 37.5 minerals |
| Build time | 4 seconds |
| Beam lock range | 112 units |
| Beam break range | 200 units |

**Controls** (per tug, per tick):
- `thrust`: [f32; 2] — 2D thrust vector (clamped to max magnitude)
- `beam_target`: Option\<EntityId\> — asteroid to grab (None to release)

**Tractor beam physics**: When locked on, a spring-damper force pulls the asteroid toward the tug (desired distance 25 units). The beam breaks if distance exceeds 200 units. Tugs can only carry one asteroid at a time. The carried asteroid is identified in the `TugView.carrying` field.

### Bullet

| Stat | Value |
|------|-------|
| Speed | 500 units/s (plus rocket's velocity) |
| Damage | 50 HP |
| Lifetime | 1.5 seconds |
| Cooldown | 0.2 seconds between shots |
| Friendly fire | Yes (damages own team's units) |

Bullets spawn from the rocket's nose and inherit the rocket's velocity plus 500 units/s in the facing direction.

## Asteroids

Six tiers of asteroids populate the field. Large asteroids fracture into smaller ones when destroyed.

| Tier | Radius | Health | Initial count | Mineral value |
|------|--------|--------|---------------|---------------|
| 1 | 10 | 125 | 120 | 25 |
| 2 | 20 | 250 | 80 | 50 |
| 3 | 40 | 500 | 40 | — |
| 4 | 80 | 1000 | 20 | — |
| 5 | 160 | 2500 | 64 | — |
| 6 | 320 | 5000 | 32 | — |

- **Gathering**: Only tier 1-2 asteroids can be collected for minerals. Tugs carry them to the station, and the station's tractor beam pulls them in.
- **Fracturing**: When a tier 3+ asteroid is destroyed, it splits into 2-4 children of the next smaller tier.
- **Mining strategy**: Shoot large asteroids to fracture them down to gatherable size, then use tugs to bring fragments to your station.
- **Mass**: `radius^2 * 0.1`

## Economy

- **Starting minerals**: 200 per team
- **Income**: Minerals earned when tier 1-2 asteroids reach the station (25 or 50 minerals)
- **Spending**: Building units (rockets 50, tugs 37.5) and station self-repair (2/HP)

## Combat

- **Bullets damage everything**: rockets, tugs, stations, asteroids (including your own units)
- **Collision damage**: Units take damage from physical collisions proportional to impact speed (factor 0.1)
- **Elastic collisions**: Units bounce off each other and off asteroids based on mass ratios
- **Station bounce**: Units colliding with stations experience partially elastic bounce (0.8 energy retention)
- **Bullet deflection**: Station tractor beams can target and deflect enemy bullets. Use `beam_targets` with a perpendicular `force_direction` for maximum deflection. Beams require 0.1s acquisition time per new target.

## Win Condition

The game ends when either station's health reaches zero. The team whose station survives wins. If both stations are destroyed on the same tick, it's a draw.

## Physics

All objects follow Newtonian mechanics:
- No friction or drag
- Velocity persists until changed by thrust or collision
- Toroidal wrapping on all edges (objects going off one edge appear on the opposite side)
- Collision detection: circle broadphase + SAT (Separating Axis Theorem) polygon narrowphase
- All shapes are wireframe polygons defined by vertices

## Coordinate System

- Origin (0, 0) is the center of the map
- X increases to the right
- Y increases upward
- Rotation 0 points up (+Y), increases counterclockwise
- `rocket.forward()` returns the unit vector the rocket is facing
- Use `state.shortest_delta(from, to)` and `state.distance(from, to)` for toroidal-aware calculations — do NOT subtract positions directly

## Config

All gameplay constants are in `config.toml` (TOML format). Edit and restart to change game dynamics. Delete the file to use built-in defaults. The config is also available to AIs via `init(&mut self, config: &GameConfig, team: Team)`.

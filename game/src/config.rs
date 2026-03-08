use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

/// All gameplay constants in one place. Can be loaded from TOML or constructed with defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
#[serde(default)]
pub struct GameConfig {
    pub world: WorldConfig,
    pub rocket: RocketConfig,
    pub tug: TugConfig,
    pub station: StationConfig,
    pub asteroids: AsteroidConfig,
    pub economy: EconomyConfig,
    pub bullet: BulletConfig,
    pub physics: PhysicsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldConfig {
    pub width: f32,
    pub height: f32,
    pub tick_rate_hz: f64,
    pub rng_seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RocketConfig {
    pub mass: f32,
    pub health: f32,
    pub max_thrust: f32,
    pub rotation_speed: f32,
    /// Vertices for the A-frame rocket shape
    pub vertices: Vec<[f32; 2]>,
    /// Line segments as pairs of vertex indices
    pub lines: Vec<[usize; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TugConfig {
    pub mass: f32,
    pub health: f32,
    pub max_thrust: f32,
    pub rotation_speed: f32,
    pub half_size: f32,
    pub beam_strength: f32,
    pub beam_desired_distance: f32,
    pub beam_lock_range: f32,
    pub beam_break_range: f32,
    pub beam_damping: f32,
    pub desired_speed: f32,
    pub station_drop_radius: f32,
    pub station_avoid_radius: f32,
    pub avoidance_margin: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StationConfig {
    pub mass: f32,
    pub health: f32,
    pub radius: f32,
    pub num_sides: usize,
    pub beam_radius: f32,
    pub beam_count: usize,
    pub pull_strength: f32,
    pub repel_strength: f32,
    pub tangential_damping: f32,
    pub radial_braking: f32,
    pub max_inbound_speed: f32,
    pub repair_rate: f32,
    pub gather_speed_threshold: f32,
    pub spawn_offset: f32,
    /// Station self-repair rate in HP/s (costs resources)
    pub self_repair_rate: f32,
    /// Resource cost per HP of self-repair
    pub repair_cost_per_hp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AsteroidTierConfig {
    pub radius: f32,
    pub vertices: usize,
    pub health: f32,
    pub spawn_count: usize,
    pub speed_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AsteroidConfig {
    /// Tier configs indexed by tier number (index 0 = tier 1, etc.)
    pub tiers: Vec<AsteroidTierConfig>,
    /// Mass = radius^2 * mass_factor
    pub mass_factor: f32,
    pub max_initial_speed: f32,
    pub max_initial_spin: f32,
    pub shape_angle_jitter: f32,
    pub shape_radius_min: f32,
    pub shape_radius_max: f32,
    pub fracture_children_min: usize,
    pub fracture_children_max: usize,
    pub fracture_spread_speed: f32,
    pub fracture_child_spin: f32,
    /// Maximum gatherable tier (tugs can pick up asteroids of this tier or lower)
    pub max_gatherable_tier: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EconomyConfig {
    pub starting_minerals: f32,
    pub rocket_cost: f32,
    pub rocket_build_time: f32,
    pub tug_cost: f32,
    pub tug_build_time: f32,
    /// Mineral values per tier (index 0 = tier 1)
    pub mineral_values: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BulletConfig {
    pub mass: f32,
    pub speed: f32,
    pub lifetime: f32,
    pub damage: f32,
    pub half_size: f32,
    pub spawn_offset: f32,
    pub shoot_cooldown: f32,
    /// If true, bullets damage friendly units too
    pub friendly_fire: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsConfig {
    /// Damage multiplier for rocket-asteroid collisions (damage = impact_speed * this)
    pub collision_damage_factor: f32,
    /// Energy retention on station bounces (1.0 = elastic, 0.0 = full stop)
    pub station_bounce_energy: f32,
    /// Minimum relative speed for collision dust
    pub dust_speed_threshold: f32,
}

// ---- Defaults ----

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            world: WorldConfig::default(),
            rocket: RocketConfig::default(),
            tug: TugConfig::default(),
            station: StationConfig::default(),
            asteroids: AsteroidConfig::default(),
            economy: EconomyConfig::default(),
            bullet: BulletConfig::default(),
            physics: PhysicsConfig::default(),
        }
    }
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            width: 10000.0,
            height: 10000.0,
            tick_rate_hz: 60.0,
            rng_seed: 42,
        }
    }
}

impl Default for RocketConfig {
    fn default() -> Self {
        Self {
            mass: 5.0,
            health: 100.0,
            max_thrust: 250.0,
            rotation_speed: 4.0,
            vertices: vec![
                [0.0, 14.0],    // tip
                [-8.0, -10.0],  // bottom left
                [8.0, -10.0],   // bottom right
                [-5.5, -6.0],   // crossbar left
                [5.5, -6.0],    // crossbar right
            ],
            lines: vec![[0, 1], [0, 2], [3, 4]],
        }
    }
}

impl Default for TugConfig {
    fn default() -> Self {
        Self {
            mass: 10.0,
            health: 60.0,
            max_thrust: 100.0,
            rotation_speed: 4.0,
            half_size: 16.0,
            beam_strength: 120.0,
            beam_desired_distance: 25.0,
            beam_lock_range: 112.0,
            beam_break_range: 200.0,
            beam_damping: 30.0,
            desired_speed: 150.0,
            station_drop_radius: 350.0,
            station_avoid_radius: 400.0,
            avoidance_margin: 25.0,
        }
    }
}

impl Default for StationConfig {
    fn default() -> Self {
        Self {
            mass: 1000.0,
            health: 500.0,
            radius: 80.0,
            num_sides: 8,
            beam_radius: 320.0,
            beam_count: 5,
            pull_strength: 500.0,
            repel_strength: 600.0,
            tangential_damping: 0.05,
            radial_braking: 0.1,
            max_inbound_speed: 30.0,
            repair_rate: 5.0,
            gather_speed_threshold: 40.0,
            spawn_offset: 100.0,
            self_repair_rate: 2.0,
            repair_cost_per_hp: 2.0,
        }
    }
}

impl Default for AsteroidTierConfig {
    fn default() -> Self {
        Self {
            radius: 10.0,
            vertices: 8,
            health: 125.0,
            spawn_count: 60,
            speed_scale: 1.0,
        }
    }
}

impl Default for AsteroidConfig {
    fn default() -> Self {
        Self {
            tiers: vec![
                AsteroidTierConfig { radius: 10.0, vertices: 8, health: 125.0, spawn_count: 60, speed_scale: 1.0 },
                AsteroidTierConfig { radius: 20.0, vertices: 10, health: 250.0, spawn_count: 40, speed_scale: 1.0 },
                AsteroidTierConfig { radius: 40.0, vertices: 12, health: 500.0, spawn_count: 15, speed_scale: 1.0 },
                AsteroidTierConfig { radius: 80.0, vertices: 16, health: 1000.0, spawn_count: 8, speed_scale: 0.5 },
                AsteroidTierConfig { radius: 160.0, vertices: 22, health: 2500.0, spawn_count: 4, speed_scale: 0.3 },
                AsteroidTierConfig { radius: 320.0, vertices: 28, health: 5000.0, spawn_count: 2, speed_scale: 0.15 },
            ],
            mass_factor: 0.1,
            max_initial_speed: 30.0,
            max_initial_spin: 1.0,
            shape_angle_jitter: 0.15,
            shape_radius_min: 0.7,
            shape_radius_max: 1.3,
            fracture_children_min: 2,
            fracture_children_max: 4,
            fracture_spread_speed: 20.0,
            fracture_child_spin: 2.0,
            max_gatherable_tier: 2,
        }
    }
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            starting_minerals: 200.0,
            rocket_cost: 100.0,
            rocket_build_time: 5.0,
            tug_cost: 75.0,
            tug_build_time: 4.0,
            mineral_values: vec![25.0, 50.0],
        }
    }
}

impl Default for BulletConfig {
    fn default() -> Self {
        Self {
            mass: 0.1,
            speed: 500.0,
            lifetime: 1.5,
            damage: 50.0,
            half_size: 1.0,
            spawn_offset: 15.0,
            shoot_cooldown: 0.2,
            friendly_fire: true,
        }
    }
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            collision_damage_factor: 0.1,
            station_bounce_energy: 0.8,
            dust_speed_threshold: 10.0,
        }
    }
}

impl GameConfig {
    pub fn load_or_default(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("Warning: failed to parse config {path}: {e}, using defaults");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save_default(path: &str) {
        let config = Self::default();
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");
        std::fs::write(path, toml_str).expect("Failed to write config file");
    }
}

// Helper methods for looking up tier info from config
impl AsteroidConfig {
    /// Get tier config by 1-based tier number. Returns default for out-of-range.
    pub fn tier(&self, tier: u8) -> &AsteroidTierConfig {
        self.tiers.get((tier as usize).saturating_sub(1))
            .unwrap_or(&self.tiers[0])
    }

    pub fn mass_for_tier(&self, tier: u8) -> f32 {
        let r = self.tier(tier).radius;
        r * r * self.mass_factor
    }
}

impl EconomyConfig {
    /// Get mineral value for a 1-based tier. Returns 0 for unknown tiers.
    pub fn mineral_value(&self, tier: u8) -> f32 {
        self.mineral_values.get((tier as usize).saturating_sub(1))
            .copied()
            .unwrap_or(0.0)
    }
}

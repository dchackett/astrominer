use bevy::prelude::*;
use crate::engine::units::components::*;
use crate::engine::units::game_rules::GameOverState;
use crate::engine::physics::components::Health;

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct RedResourceText;

#[derive(Component)]
pub struct BlueResourceText;

#[derive(Component)]
pub struct TickText;

#[derive(Component)]
pub struct GameOverText;

pub fn setup_hud(mut commands: Commands) {
    // Top-left info panel
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                TickText,
                Text::new("Tick: 0"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
            parent.spawn((
                RedResourceText,
                Text::new("Red: 200"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(1.0, 0.3, 0.3)),
            ));
            parent.spawn((
                BlueResourceText,
                Text::new("Blue: 200"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.3, 0.5, 1.0)),
            ));
        });

    // Game over overlay (hidden until game ends)
    commands.spawn((
        GameOverText,
        Text::new(""),
        TextFont { font_size: 48.0, ..default() },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(40.0),
            left: Val::Percent(30.0),
            ..default()
        },
    ));
}

pub fn update_hud(
    resources: Res<TeamResources>,
    player_ais: Res<PlayerAIs>,
    tick: Res<TickCounter>,
    stations: Query<(&Team, Option<&BuildProgress>, &BuildQueue, &Health), With<Station>>,
    rockets: Query<&Team, With<Rocket>>,
    tugs: Query<&Team, With<Tug>>,
    game_over: Res<GameOverState>,
    mut tick_text: Query<&mut Text, (With<TickText>, Without<RedResourceText>, Without<BlueResourceText>, Without<GameOverText>)>,
    mut red_text: Query<&mut Text, (With<RedResourceText>, Without<TickText>, Without<BlueResourceText>, Without<GameOverText>)>,
    mut blue_text: Query<&mut Text, (With<BlueResourceText>, Without<TickText>, Without<RedResourceText>, Without<GameOverText>)>,
    mut game_over_text: Query<&mut Text, (With<GameOverText>, Without<TickText>, Without<RedResourceText>, Without<BlueResourceText>)>,
) {
    if let Ok(mut text) = tick_text.single_mut() {
        let secs = tick.0 as f32 / 60.0;
        **text = format!("Tick: {} ({:.0}s)", tick.0, secs);
    }

    // Count units per team
    let mut red_rockets = 0u32;
    let mut blue_rockets = 0u32;
    let mut red_tugs = 0u32;
    let mut blue_tugs = 0u32;
    for team in &rockets {
        match team { Team::Red => red_rockets += 1, Team::Blue => blue_rockets += 1 }
    }
    for team in &tugs {
        match team { Team::Red => red_tugs += 1, Team::Blue => blue_tugs += 1 }
    }

    let red_name = player_ais.red.name();
    let blue_name = player_ais.blue.name();

    let mut red_info = String::new();
    let mut blue_info = String::new();

    for (team, progress, queue, health) in &stations {
        let minerals = resources.minerals(*team);
        let (r_count, t_count, ai_name) = match team {
            Team::Red => (red_rockets, red_tugs, red_name),
            Team::Blue => (blue_rockets, blue_tugs, blue_name),
        };
        let mut s = format!(
            "{:?} ({}): {:.0} minerals | HP: {:.0}/{:.0} | R:{} T:{}",
            team, ai_name, minerals, health.current, health.max, r_count, t_count
        );

        if let Some(bp) = progress {
            let name = match bp.unit_type {
                UnitType::Rocket => "Rocket",
                UnitType::Tug => "Tug",
            };
            let pct = (bp.progress / bp.total * 100.0) as u32;
            s.push_str(&format!(" | Building {name}: {pct}%"));
        }
        if !queue.0.is_empty() {
            s.push_str(&format!(" | Queue: {}", queue.0.len()));
        }

        match team {
            Team::Red => red_info = s,
            Team::Blue => blue_info = s,
        }
    }

    if let Ok(mut text) = red_text.single_mut() {
        **text = red_info;
    }
    if let Ok(mut text) = blue_text.single_mut() {
        **text = blue_info;
    }

    // Game over overlay
    if let Ok(mut text) = game_over_text.single_mut() {
        if let Some(ref result) = game_over.result {
            let winner_str = match result.winner {
                Some(crate::api::Team::Red) => "RED WINS!",
                Some(crate::api::Team::Blue) => "BLUE WINS!",
                None => "DRAW!",
            };
            **text = format!("{}\n{}", winner_str, result.reason);
        }
    }
}

/// Draw health bars above all units.
pub fn render_health_bars(
    units: Query<(&Transform, &Health, Option<&Team>), Without<crate::engine::asteroids::components::Asteroid>>,
    mut gizmos: Gizmos,
) {
    for (tf, health, team) in &units {
        if health.current >= health.max { continue; } // Don't show for full-health units
        if health.current <= 0.0 { continue; }

        let pos = tf.translation.truncate();
        let bar_y = pos.y + 20.0; // Above the unit
        let bar_width = 20.0;
        let bar_height = 2.0;
        let health_frac = (health.current / health.max).clamp(0.0, 1.0);

        // Background (dark)
        let left = pos.x - bar_width / 2.0;
        let right = pos.x + bar_width / 2.0;
        gizmos.line_2d(
            Vec2::new(left, bar_y),
            Vec2::new(right, bar_y),
            Color::srgba(0.3, 0.3, 0.3, 0.6),
        );

        // Health fill
        let fill_right = left + bar_width * health_frac;
        let color = if health_frac > 0.5 {
            Color::srgb(0.0, 1.0, 0.0)
        } else if health_frac > 0.25 {
            Color::srgb(1.0, 1.0, 0.0)
        } else {
            Color::srgb(1.0, 0.0, 0.0)
        };
        gizmos.line_2d(Vec2::new(left, bar_y), Vec2::new(fill_right, bar_y), color);
    }
}

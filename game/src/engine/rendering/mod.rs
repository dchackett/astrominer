use bevy::prelude::*;
use bevy::mesh::PrimitiveTopology;
use bevy::asset::RenderAssetUsages;

use crate::engine::physics::components::*;
use crate::engine::units::components::{Team, Rocket, Station, Tug};
use crate::engine::rendering::camera::GameCamera;

pub mod camera;
pub mod dust;
pub mod hud;
pub mod rts_visuals;

#[derive(Component)]
pub struct WireframeMesh;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (camera::setup_camera, hud::setup_hud))
            .add_systems(
                Update,
                (
                    sync_wireframe_meshes,
                    camera::camera_follow,
                    camera::camera_zoom,
                    render_thruster_flame,
                    dust::tick_dust_particles,
                    dust::render_dust_particles,
                ),
            )
            .add_systems(
                Update,
                (
                    rts_visuals::render_station_beam_range,
                    rts_visuals::render_station_beam,
                    rts_visuals::render_tractor_beams,
                    rts_visuals::render_station_ai_beams,
                    rts_visuals::tug_exhaust_particles,
                    hud::update_hud,
                    hud::render_health_bars,
                    zoom_adaptive_visibility,
                    render_zoomed_out_icons,
                ),
            );
    }
}

/// Create/update wireframe meshes from WireframeShape components.
fn sync_wireframe_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(Entity, &WireframeShape, Option<&Team>), Without<WireframeMesh>>,
) {
    for (entity, shape, team) in &query {
        if shape.vertices.is_empty() {
            continue;
        }

        let mesh = if shape.lines.is_empty() {
            let mut positions: Vec<[f32; 3]> = shape
                .vertices
                .iter()
                .map(|v| [v.x, v.y, 0.0])
                .collect();
            if let Some(first) = shape.vertices.first() {
                positions.push([first.x, first.y, 0.0]);
            }
            let mut m = Mesh::new(PrimitiveTopology::LineStrip, RenderAssetUsages::default());
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            m
        } else {
            let mut positions: Vec<[f32; 3]> = Vec::new();
            for &[a, b] in &shape.lines {
                if a < shape.vertices.len() && b < shape.vertices.len() {
                    positions.push([shape.vertices[a].x, shape.vertices[a].y, 0.0]);
                    positions.push([shape.vertices[b].x, shape.vertices[b].y, 0.0]);
                }
            }
            let mut m = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            m
        };

        // Color by team
        let color = match team {
            Some(Team::Red) => Color::srgb(1.0, 0.3, 0.3),
            Some(Team::Blue) => Color::srgb(0.3, 0.5, 1.0),
            None => Color::WHITE,
        };

        commands.entity(entity).insert((
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            WireframeMesh,
        ));
    }
}

/// When zoomed out past a threshold, hide wireframe meshes so icons can take over.
fn zoom_adaptive_visibility(
    camera: Query<&Projection, With<GameCamera>>,
    mut wireframes: Query<(&mut Visibility, Option<&Rocket>, Option<&Tug>, Option<&Station>), With<WireframeMesh>>,
) {
    let Ok(proj) = camera.single() else { return };
    let scale = match proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    // Hide wireframe details when a rocket (~20px) would be < 3 pixels on screen
    let icon_threshold = 7.0;

    for (mut vis, is_rocket, is_tug, is_station) in &mut wireframes {
        if is_station.is_some() {
            // Stations are big enough to always show
            continue;
        }
        if (is_rocket.is_some() || is_tug.is_some()) && scale > icon_threshold {
            *vis = Visibility::Hidden;
        } else {
            *vis = Visibility::Inherited;
        }
    }
}

/// Draw simple colored icons for units when zoomed out far.
fn render_zoomed_out_icons(
    camera: Query<&Projection, With<GameCamera>>,
    rockets: Query<(&Transform, &Team), With<Rocket>>,
    tugs: Query<(&Transform, &Team), With<Tug>>,
    stations: Query<(&Transform, &Team), With<Station>>,
    mut gizmos: Gizmos,
) {
    let Ok(proj) = camera.single() else { return };
    let scale = match proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };

    let icon_threshold = 7.0;
    if scale <= icon_threshold { return; }

    // Icon size scales with zoom so they stay visible
    let icon_size = scale * 3.0;

    for (tf, team) in &rockets {
        let pos = tf.translation.truncate();
        let color = match team {
            Team::Red => Color::srgb(1.0, 0.3, 0.3),
            Team::Blue => Color::srgb(0.3, 0.5, 1.0),
        };
        // Draw a small diamond for rockets
        let up = Vec2::new(0.0, icon_size);
        let right = Vec2::new(icon_size * 0.6, 0.0);
        gizmos.line_2d(pos + up, pos + right, color);
        gizmos.line_2d(pos + right, pos - up, color);
        gizmos.line_2d(pos - up, pos - right, color);
        gizmos.line_2d(pos - right, pos + up, color);
    }

    for (tf, team) in &tugs {
        let pos = tf.translation.truncate();
        let color = match team {
            Team::Red => Color::srgb(1.0, 0.5, 0.3),
            Team::Blue => Color::srgb(0.3, 0.7, 1.0),
        };
        // Draw a small square for tugs
        let hs = icon_size * 0.5;
        gizmos.line_2d(pos + Vec2::new(-hs, -hs), pos + Vec2::new(hs, -hs), color);
        gizmos.line_2d(pos + Vec2::new(hs, -hs), pos + Vec2::new(hs, hs), color);
        gizmos.line_2d(pos + Vec2::new(hs, hs), pos + Vec2::new(-hs, hs), color);
        gizmos.line_2d(pos + Vec2::new(-hs, hs), pos + Vec2::new(-hs, -hs), color);
    }

    for (tf, team) in &stations {
        let pos = tf.translation.truncate();
        let color = match team {
            Team::Red => Color::srgb(1.0, 0.3, 0.3),
            Team::Blue => Color::srgb(0.3, 0.5, 1.0),
        };
        // Draw a large cross for stations
        let s = icon_size * 2.0;
        gizmos.line_2d(pos + Vec2::new(-s, 0.0), pos + Vec2::new(s, 0.0), color);
        gizmos.line_2d(pos + Vec2::new(0.0, -s), pos + Vec2::new(0.0, s), color);
    }
}

/// Render flickering thruster flame when thrust is active.
fn render_thruster_flame(
    query: Query<(&Transform, &Thrust, Option<&Team>), With<Rocket>>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    for (transform, thrust, team) in &query {
        if thrust.forward > 0.0 {
            let pos = transform.translation.truncate();
            let backward = -transform.local_y().truncate();
            let right = transform.local_x().truncate();

            let t = time.elapsed_secs();
            let flicker = ((t * 30.0).sin() * 0.5 + 0.5) * 0.6 + 0.4;
            let flicker2 = ((t * 47.0).cos() * 0.5 + 0.5) * 0.5 + 0.5;

            let base_len = 6.0 + (thrust.forward / thrust.max_forward) * 12.0;
            let flame_len = base_len * flicker;
            let width = 3.0 * flicker2;

            let base = pos + backward * 8.0;
            let tip = base + backward * flame_len;
            let left = base + right * width;
            let right_pt = base - right * width;

            let flame_color = match team {
                Some(Team::Red) => Color::srgb(1.0, 0.5 + 0.3 * flicker, 0.1 + 0.2 * flicker2),
                Some(Team::Blue) => Color::srgb(0.1 + 0.2 * flicker2, 0.5 + 0.3 * flicker, 1.0),
                _ => Color::srgb(1.0, 0.7 + 0.3 * flicker, 0.1 + 0.2 * flicker2),
            };
            gizmos.line_2d(left, tip, flame_color);
            gizmos.line_2d(right_pt, tip, flame_color);
            gizmos.line_2d(left, right_pt, flame_color);
        }
    }
}

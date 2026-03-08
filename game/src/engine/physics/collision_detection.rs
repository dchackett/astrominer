use bevy::prelude::*;
use super::components::*;
use super::toroidal::WorldBounds;

/// A detected collision between two entities.
#[derive(Clone)]
pub struct Collision {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub normal: Vec2,
    pub overlap: f32,
}

/// Collision list built each physics tick.
#[derive(Resource, Default)]
pub struct Collisions(pub Vec<Collision>);

/// Collision detection: circle broadphase, polygon SAT narrowphase.
pub fn detect_collisions(
    query: Query<(Entity, &Transform, &WireframeShape)>,
    bounds: Res<WorldBounds>,
    mut collisions: ResMut<Collisions>,
) {
    collisions.0.clear();

    let entities: Vec<_> = query.iter().collect();
    for i in 0..entities.len() {
        let (entity_a, transform_a, shape_a) = entities[i];
        let radius_a = bounding_radius(shape_a);
        let pos_a = transform_a.translation.truncate();

        for j in (i + 1)..entities.len() {
            let (entity_b, transform_b, shape_b) = entities[j];
            let radius_b = bounding_radius(shape_b);
            let pos_b = transform_b.translation.truncate();

            let delta = bounds.shortest_delta(pos_a, pos_b);
            let dist_sq = delta.length_squared();
            let min_dist = radius_a + radius_b;

            // Circle broadphase
            if dist_sq >= min_dist * min_dist || dist_sq <= 0.0 {
                continue;
            }

            // Transform vertices to world space
            let verts_a = world_vertices(shape_a, transform_a);
            let verts_b_raw = world_vertices(shape_b, transform_b);
            // Offset B's vertices by the toroidal delta so they're in A's frame
            let offset = delta - (pos_b - pos_a);
            let verts_b: Vec<Vec2> = verts_b_raw.iter().map(|v| *v + offset).collect();

            if let Some((normal, overlap)) = sat_collision(&verts_a, &verts_b) {
                collisions.0.push(Collision {
                    entity_a,
                    entity_b,
                    normal,
                    overlap,
                });
            }
        }
    }
}

/// Get polygon vertices in world space (rotated + translated).
fn world_vertices(shape: &WireframeShape, transform: &Transform) -> Vec<Vec2> {
    let pos = transform.translation.truncate();
    let rot = transform.rotation.to_euler(EulerRot::ZYX).0;
    let cos = rot.cos();
    let sin = rot.sin();
    shape.vertices.iter().map(|v| {
        let rotated = Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
        pos + rotated
    }).collect()
}

/// SAT collision test between two convex polygons.
/// Returns Some((normal, overlap)) pointing from A toward B, or None if separated.
fn sat_collision(verts_a: &[Vec2], verts_b: &[Vec2]) -> Option<(Vec2, f32)> {
    if verts_a.len() < 2 || verts_b.len() < 2 {
        return None;
    }

    let mut min_overlap = f32::MAX;
    let mut min_axis = Vec2::ZERO;

    for verts in [verts_a, verts_b] {
        let n = verts.len();
        for i in 0..n {
            let edge = verts[(i + 1) % n] - verts[i];
            let axis = Vec2::new(-edge.y, edge.x);
            let axis_len = axis.length();
            if axis_len < 0.001 { continue; }
            let axis = axis / axis_len;

            let (min_a, max_a) = project_polygon(verts_a, axis);
            let (min_b, max_b) = project_polygon(verts_b, axis);

            let overlap = (max_a.min(max_b)) - (min_a.max(min_b));
            if overlap <= 0.0 {
                return None;
            }

            if overlap < min_overlap {
                min_overlap = overlap;
                min_axis = axis;
            }
        }
    }

    // Ensure normal points from A toward B
    let center_a: Vec2 = verts_a.iter().copied().sum::<Vec2>() / verts_a.len() as f32;
    let center_b: Vec2 = verts_b.iter().copied().sum::<Vec2>() / verts_b.len() as f32;
    if min_axis.dot(center_b - center_a) < 0.0 {
        min_axis = -min_axis;
    }

    Some((min_axis, min_overlap))
}

/// Project a polygon onto an axis, returning (min, max).
fn project_polygon(verts: &[Vec2], axis: Vec2) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for v in verts {
        let proj = v.dot(axis);
        min = min.min(proj);
        max = max.max(proj);
    }
    (min, max)
}

pub fn bounding_radius(shape: &WireframeShape) -> f32 {
    shape
        .vertices
        .iter()
        .map(|v| v.length())
        .fold(0.0f32, f32::max)
}

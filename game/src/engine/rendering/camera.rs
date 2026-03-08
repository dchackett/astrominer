use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::config::GameConfig;

#[derive(Component)]
pub struct GameCamera;

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        GameCamera,
    ));
}

/// Camera follows the center of the map (observer mode for programming game).
/// Can be moved with arrow keys / WASD.
pub fn camera_follow(
    mut camera: Query<&mut Transform, With<GameCamera>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    projection: Query<&Projection, With<GameCamera>>,
) {
    let Ok(mut cam_transform) = camera.single_mut() else { return };
    let Ok(proj) = projection.single() else { return };

    let speed = 500.0;
    let scale = match proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let move_speed = speed * scale * time.delta_secs();

    let mut delta = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        delta.y += move_speed;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        delta.y -= move_speed;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        delta.x -= move_speed;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        delta.x += move_speed;
    }

    cam_transform.translation.x += delta.x;
    cam_transform.translation.y += delta.y;
}

/// Zoom in/out with mouse wheel. Max zoom adapts to world size.
pub fn camera_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    mut camera: Query<&mut Projection, With<GameCamera>>,
    config: Res<GameConfig>,
) {
    let Ok(mut projection) = camera.single_mut() else { return };
    let Projection::Orthographic(ref mut ortho) = *projection else { return };

    // Max zoom out: enough to see the entire world height in a 720p window
    let max_scale = config.world.height / 720.0;

    for event in scroll_events.read() {
        let zoom_speed = 0.015;
        ortho.scale *= 1.0 - event.y * zoom_speed;
        ortho.scale = ortho.scale.clamp(0.2, max_scale);
    }
}

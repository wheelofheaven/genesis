use bevy::input::mouse::{AccumulatedMouseMotion, MouseWheel};
use bevy::prelude::*;

/// Orbit camera that rotates around a focus point.
#[derive(Component)]
pub struct OrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sensitivity: f32,
    pub zoom_speed: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 800.0,
            yaw: std::f32::consts::FRAC_PI_4,
            pitch: std::f32::consts::FRAC_PI_4,
            sensitivity: 0.005,
            zoom_speed: 30.0,
        }
    }
}

/// Reads right-click drag for rotation and scroll wheel for zoom.
pub fn orbit_camera_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut scroll: MessageReader<MouseWheel>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    let Ok((mut orbit, mut transform)) = query.single_mut() else {
        return;
    };

    // Rotate on right-click drag.
    if mouse_button.pressed(MouseButton::Right) {
        let delta = mouse_motion.delta;
        orbit.yaw -= delta.x * orbit.sensitivity;
        orbit.pitch -= delta.y * orbit.sensitivity;
    }

    // Clamp pitch to avoid gimbal lock.
    orbit.pitch = orbit.pitch.clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.05);

    // Zoom with scroll wheel.
    for event in scroll.read() {
        orbit.radius -= event.y * orbit.zoom_speed;
    }
    orbit.radius = orbit.radius.clamp(350.0, 2000.0);

    // Compute camera position from spherical coordinates.
    let x = orbit.radius * orbit.pitch.cos() * orbit.yaw.cos();
    let z = orbit.radius * orbit.pitch.cos() * orbit.yaw.sin();
    let y = orbit.radius * orbit.pitch.sin();

    let eye = orbit.focus + Vec3::new(x, y, z);
    *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
}

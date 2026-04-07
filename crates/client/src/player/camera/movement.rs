use bevy::{
    ecs::prelude::*,
    input::mouse::{MouseMotion, MouseWheel},
    math::{Mat4, Vec3},
    window::{PrimaryWindow, Window, WindowResized},
};
use shared::simulation::{
    chunk::ChunkCoord,
    input::{resources::ActionStateResource, types::SimulationAction},
    player::{active_camera::ActiveCamera, camera_component::CameraComponent},
    time::FrameClock,
};
use tracing::{debug, instrument, warn};

/// The distance the near plane is set to for the camera frustum.
pub const CAMERA_NEAR_PLANE: f32 = 1.0;
const MOVEMENT_SPEED: f32 = 16.0;
const MOUSE_SENSITIVITY: f32 = 0.1;

/// A system that updates the active camera's position and orientation based on user input.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn camera_movement_system(
    // Input
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut resize_events: MessageReader<WindowResized>,
    action_state: Res<ActionStateResource>,
    time: Res<FrameClock>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    active_camera: Res<ActiveCamera>,

    // Output
    mut camera_query: Query<&mut CameraComponent>,
) {
    let Ok(mut cam) = camera_query.get_mut(active_camera.0) else {
        warn!(
            "camera_control_system: ActiveCamera entity {:?} not found or has no CameraComponent.",
            active_camera.0
        );
        return;
    };

    // retrieve the primary window to calculate the aspect ratio
    let Ok(window) = window_query.single() else {
        return;
    };

    // prevent division by zero if the window is minimized or just spawning
    let aspect_ratio = (window.width() / window.height()).max(0.0001);

    // check if the window was resized this frame
    let mut window_resized = false;
    for _ in resize_events.read() {
        window_resized = true;
    }

    // update position based on input
    let velocity = MOVEMENT_SPEED * time.delta.as_secs_f32();
    let front = cam.front;
    let mut multiplier = 1.0;

    if action_state.is_ongoing(SimulationAction::MoveFaster) {
        multiplier = 2.5;
    }
    if action_state.is_ongoing(SimulationAction::MoveForward) {
        cam.position += front * velocity * multiplier;
    }
    let front = cam.front;
    if action_state.is_ongoing(SimulationAction::MoveBackward) {
        cam.position -= front * velocity * multiplier;
    }
    let right = cam.right;
    if action_state.is_ongoing(SimulationAction::MoveLeft) {
        cam.position -= right * velocity * multiplier;
    }
    let right = cam.right;
    if action_state.is_ongoing(SimulationAction::MoveRight) {
        cam.position += right * velocity * multiplier;
    }

    // update rotation using native Bevy mouse motion events
    let mut xoffset = 0.0;
    let mut yoffset = 0.0;
    for ev in mouse_motion.read() {
        xoffset += ev.delta.x;
        yoffset += ev.delta.y;
    }

    xoffset *= MOUSE_SENSITIVITY;
    yoffset *= MOUSE_SENSITIVITY;

    cam.yaw += xoffset;
    cam.pitch -= yoffset;

    cam.pitch = cam.pitch.clamp(-89.0, 89.0);

    // update internal vectors
    let yaw_radians = cam.yaw.to_radians();
    let pitch_radians = cam.pitch.to_radians();

    let x = yaw_radians.cos() * pitch_radians.cos();
    let y = pitch_radians.sin();
    let z = yaw_radians.sin() * pitch_radians.cos();

    cam.front = Vec3::new(x, y, z).normalize();
    cam.right = cam.front.cross(cam.world_up).normalize();
    cam.up = cam.right.cross(cam.front).normalize();

    // handle zoom using native Bevy mouse wheel events
    let mut yoffset_scroll = 0.0;
    for ev in mouse_wheel.read() {
        yoffset_scroll += ev.y;
    }

    let mut zoom_changed = false;
    if yoffset_scroll != 0.0 {
        cam.zoom -= yoffset_scroll;
        cam.zoom = cam.zoom.clamp(1.0, 45.0);
        zoom_changed = true;
    }

    // updated matrices
    cam.view_matrix = Mat4::look_at_rh(cam.position, cam.position + cam.front, cam.up);

    // ensure projection is always up-to-date (esp. on first frame or if IDENTITY)
    if zoom_changed || window_resized || cam.projection_matrix == Mat4::IDENTITY {
        cam.projection_matrix = Mat4::perspective_infinite_reverse_rh(
            cam.zoom.to_radians(),
            aspect_ratio,
            CAMERA_NEAR_PLANE,
        );
    }
}

/// A system to that updates the active camera's chunk chord based on its position.
#[instrument(skip_all)]
pub fn update_camera_chunk_chord_system(
    active_camera: Res<ActiveCamera>,
    mut camera_query: Query<(&CameraComponent, &mut ChunkCoord)>,
) {
    let Ok((camera, mut vicinity)) = camera_query.get_mut(active_camera.0) else {
        warn!(
            "update_camera_chunk_vicinity_system: ActiveCamera entity {:?} has no CameraComponent or ChunkVicinity.",
            active_camera.0
        );
        return;
    };

    // update chunk chord if it is different
    let new_chunk_pos = ChunkCoord::world_to_chunk_pos(camera.position);
    if new_chunk_pos != vicinity.pos {
        debug!(
            target: "camera_chunk",
            "Camera crossed chunk boundary. Old: {:?}, New: {:?}",
            vicinity.pos, new_chunk_pos
        );
        vicinity.pos = new_chunk_pos;
    }
}

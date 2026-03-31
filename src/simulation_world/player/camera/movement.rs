use crate::prelude::*;
use crate::{
    simulation_world::chunk::ChunkCoord,
    simulation_world::input::resources::WindowSizeResource,
    simulation_world::input::{
        resources::CursorMovement, types::simulation_action::SimulationAction, ActionStateResource,
    },
    simulation_world::player::{ActiveCamera, CameraComponent},
    simulation_world::time::FrameClock,
};
use bevy::ecs::prelude::*;
use bevy::math::{Mat4, Vec3};
use tracing::{instrument, warn};

/// The distance the near plane is set to for the camera frustum.
pub const CAMERA_NEAR_PLANE: f32 = 1.0;
const MOVEMENT_SPEED: f32 = 16.0;
const MOUSE_SENSITIVITY: f32 = 0.1;

/// A system that updates the active camera's position and orientation based on user input.
#[instrument(skip_all)]
pub fn camera_movement_system(
    // Input
    movement: Res<CursorMovement>,
    action_state: Res<ActionStateResource>,
    time: Res<FrameClock>,
    window: Res<WindowSizeResource>,
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

    // update rotation
    let mut xoffset = movement.get_mouse_delta().x as f32;
    let mut yoffset = movement.get_mouse_delta().y as f32;

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

    // handle zoom
    let yoffset_scroll = movement.get_scroll_delta().y;
    let mut zoom_changed = false;

    if yoffset_scroll != 0.0 {
        cam.zoom -= yoffset_scroll;
        cam.zoom = cam.zoom.clamp(1.0, 45.0);
        zoom_changed = true;
    }

    // updated matrices
    cam.view_matrix = Mat4::look_at_rh(cam.position, cam.position + cam.front, cam.up);
    if zoom_changed || window.is_changed() {
        cam.projection_matrix = Mat4::perspective_infinite_reverse_rh(
            cam.zoom.to_radians(),
            window.aspect_ratio(),
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
        warn!("update_camera_chunk_vicinity_system: ActiveCamera entity {:?} has no CameraComponent or ChunkVicinity.", active_camera.0);
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

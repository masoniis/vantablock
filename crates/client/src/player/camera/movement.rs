use bevy::{
    ecs::prelude::*,
    input::mouse::{MouseMotion, MouseWheel},
    math::EulerRot,
    prelude::{Camera3d, Projection, Quat, Transform},
};
use shared::simulation::{
    chunk::ChunkCoord,
    input::{resources::ActionStateResource, types::SimulationAction},
    player::active_camera::ActiveCamera,
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
    // input
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    action_state: Res<ActionStateResource>,
    time: Res<FrameClock>,
    active_camera: Res<ActiveCamera>,

    // output
    mut camera_query: Query<(&mut Transform, &Camera3d, &mut Projection)>,
) {
    let Ok((mut transform, _camera, mut projection)) = camera_query.get_mut(active_camera.0) else {
        warn!(
            "camera_control_system: ActiveCamera entity {:?} not found or has no Transform/Camera3d/Projection.",
            active_camera.0
        );
        return;
    };

    // update position based on input
    let velocity = MOVEMENT_SPEED * time.delta.as_secs_f32();
    let front = transform.forward();
    let mut multiplier = 1.0;

    if action_state.is_ongoing(SimulationAction::MoveFaster) {
        multiplier = 2.5;
    }
    if action_state.is_ongoing(SimulationAction::MoveForward) {
        transform.translation += front * velocity * multiplier;
    }
    if action_state.is_ongoing(SimulationAction::MoveBackward) {
        transform.translation -= front * velocity * multiplier;
    }
    let right = transform.right();
    if action_state.is_ongoing(SimulationAction::MoveLeft) {
        transform.translation -= right * velocity * multiplier;
    }
    if action_state.is_ongoing(SimulationAction::MoveRight) {
        transform.translation += right * velocity * multiplier;
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

    // extract current yaw and pitch to prevent roll (gimbal lock handling)
    let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

    yaw -= xoffset.to_radians();
    pitch -= yoffset.to_radians();

    // clamp pitch to avoid flipping
    pitch = pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());

    // apply constrained rotation
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

    // handle zoom using native Bevy mouse wheel events
    let mut yoffset_scroll = 0.0;
    for ev in mouse_wheel.read() {
        yoffset_scroll += ev.y;
    }

    if let Projection::Perspective(ref mut perspective) = *projection
        && yoffset_scroll != 0.0
    {
        // zoom in the old system was FOV in degrees, between 1 and 45
        // here we map it to radians
        let mut current_fov_deg = perspective.fov.to_degrees();
        current_fov_deg -= yoffset_scroll;
        current_fov_deg = current_fov_deg.clamp(1.0, 45.0);
        perspective.fov = current_fov_deg.to_radians();
    }
}

/// A system to that updates the active camera's chunk chord based on its position.
#[instrument(skip_all)]
pub fn update_camera_chunk_chord_system(
    active_camera: Res<ActiveCamera>,
    mut camera_query: Query<(&Transform, &mut ChunkCoord)>,
) {
    let Ok((transform, mut vicinity)) = camera_query.get_mut(active_camera.0) else {
        warn!(
            "update_camera_chunk_vicinity_system: ActiveCamera entity {:?} has no Transform or ChunkVicinity.",
            active_camera.0
        );
        return;
    };

    // update chunk chord if it is different
    let new_chunk_pos = ChunkCoord::world_to_chunk_pos(transform.translation);
    if new_chunk_pos != vicinity.pos {
        debug!(
            target: "camera_chunk",
            "Camera crossed chunk boundary. Old: {:?}, New: {:?}",
            vicinity.pos, new_chunk_pos
        );
        vicinity.pos = new_chunk_pos;
    }
}

use crate::player::LocalPlayer;
use bevy::ecs::relationship::Relationship;
use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    math::EulerRot,
    prelude::*,
};
use shared::player::components::PlayerLook;
use shared::world::chunk::ChunkCoord;
use tracing::{debug, instrument};

/// The distance the near plane is set to for the camera frustum.
pub const CAMERA_NEAR_PLANE: f32 = 1.0;
const MOUSE_SENSITIVITY: f32 = 0.1;

/// A system that updates the active camera's position and orientation based on user input.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn camera_movement_system(
    // input
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,

    // output
    mut player_query: Query<(Entity, &mut PlayerLook, &mut Transform), With<LocalPlayer>>,
    mut camera_query: Query<(&Camera, &mut Projection, &ChildOf), With<Camera3d>>,
) {
    if player_query.is_empty() {
        return;
    }
    let (player_entity, mut look, mut transform) = player_query.single_mut().unwrap();

    // update rotation using native Bevy mouse motion events
    let mut xoffset = 0.0;
    let mut yoffset = 0.0;
    for ev in mouse_motion.read() {
        xoffset += ev.delta.x;
        yoffset += ev.delta.y;
    }

    if xoffset != 0.0 || yoffset != 0.0 {
        xoffset *= MOUSE_SENSITIVITY;
        yoffset *= MOUSE_SENSITIVITY;

        look.yaw -= xoffset.to_radians();
        look.pitch -= yoffset.to_radians();

        // clamp pitch to avoid flipping
        look.pitch = look
            .pitch
            .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());

        // apply constrained rotation to the player transform
        transform.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
    }

    // handle zoom using native Bevy mouse wheel events
    let mut yoffset_scroll = 0.0;
    for ev in mouse_wheel.read() {
        yoffset_scroll += ev.y;
    }

    if yoffset_scroll != 0.0 {
        for (camera, mut projection, parent) in camera_query.iter_mut() {
            if camera.is_active
                && parent.get() == player_entity
                && let Projection::Perspective(ref mut perspective) = *projection
            {
                let mut current_fov_deg = perspective.fov.to_degrees();
                current_fov_deg -= yoffset_scroll;
                current_fov_deg = current_fov_deg.clamp(1.0, 45.0);
                perspective.fov = current_fov_deg.to_radians();
            }
        }
    }
}

/// A system to that updates the active camera's chunk chord based on its position.
#[instrument(skip_all)]
pub fn update_camera_chunk_chord_system(mut query: Query<(&GlobalTransform, &mut ChunkCoord)>) {
    for (transform, mut vicinity) in query.iter_mut() {
        // update chunk chord if it is different
        let new_chunk_pos = ChunkCoord::world_to_chunk_pos(transform.translation());
        if new_chunk_pos != vicinity.pos {
            debug!(
                target: "camera_chunk",
                "Entity crossed chunk boundary. Old: {:?}, New: {:?}",
                vicinity.pos, new_chunk_pos
            );
            vicinity.pos = new_chunk_pos;
        }
    }
}

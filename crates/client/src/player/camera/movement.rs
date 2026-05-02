use crate::player::LocalPlayer;
use bevy::{
    ecs::relationship::Relationship,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};
use shared::{
    network::{ClientMessage, PlayerMovement},
    player::components::PlayerLook,
};
use tracing::instrument;

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
    mut player_query: Query<(Entity, &mut PlayerLook), With<LocalPlayer>>,
    mut camera_query: Query<(&Camera, &mut Projection, &ChildOf), With<Camera3d>>,
) {
    if player_query.is_empty() {
        return;
    }
    let (player_entity, mut look) = player_query.single_mut().unwrap();

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

/// Sends the player's look orientation to the server.
pub fn sync_player_look_to_server_system(
    player_query: Query<&PlayerLook, (With<LocalPlayer>, Changed<PlayerLook>)>,
    mut sender_query: Query<
        &mut lightyear::prelude::MessageSender<shared::network::protocol::ClientMessage>,
    >,
) {
    let Ok(look) = player_query.single() else {
        return;
    };

    let Ok(mut sender) = sender_query.single_mut() else {
        return;
    };

    // calculate forward vector from yaw and pitch
    let forward = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0) * -Vec3::Z;

    sender.send::<PlayerMovement>(ClientMessage::UpdateView { forward });
}

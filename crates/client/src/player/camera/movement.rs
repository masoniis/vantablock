use crate::{
    input::local_actions::ClientAction,
    player::LocalPlayer,
};
use bevy::{
    ecs::relationship::Relationship,
    input::mouse::MouseWheel,
    prelude::*,
};
use leafwing_input_manager::prelude::ActionState;
use shared::player::components::PlayerLook;
use tracing::instrument;

/// The distance the near plane is set to for the camera frustum.
pub const CAMERA_NEAR_PLANE: f32 = 1.0;
const MOUSE_SENSITIVITY: f32 = 0.1;

/// A system that handles player rotation in Update for maximum smoothness.
///
/// This system updates the authoritative `PlayerLook` component on the local player.
/// Since `PlayerLook` is registered for prediction, these changes are replicated
/// to the server.
pub fn local_camera_look_system(
    mut player_query: Query<(&ActionState<ClientAction>, &mut PlayerLook), With<LocalPlayer>>,
) {
    let Ok((action_state, mut look)) = player_query.single_mut() else {
        return;
    };

    let look_delta = action_state.axis_pair(&ClientAction::Look);

    if look_delta.x != 0.0 || look_delta.y != 0.0 {
        look.yaw -= (look_delta.x * MOUSE_SENSITIVITY).to_radians();
        look.pitch -= (look_delta.y * MOUSE_SENSITIVITY).to_radians();

        // clamp pitch to avoid flipping
        look.pitch = look.pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
    }
}

/// A system that handles the 3rd person zoom. Rotation and Position is updated via the
/// shared movement system and visual smoothing.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn camera_movement_system(
    // input
    mut mouse_wheel: MessageReader<MouseWheel>,

    // output
    mut player_query: Query<Entity, With<LocalPlayer>>,
    mut camera_query: Query<(&Camera, &mut Projection, &ChildOf), With<Camera3d>>,
) {
    if player_query.is_empty() {
        return;
    }
    let player_entity = player_query.single_mut().unwrap();

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

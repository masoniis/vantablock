use crate::{input::local_actions::ClientAction, player::LocalPlayer};
use bevy::{ecs::relationship::Relationship, prelude::*};
use leafwing_input_manager::prelude::ActionState;
use tracing::instrument;

/// The distance the near plane is set to for the camera frustum.
pub const CAMERA_NEAR_PLANE: f32 = 1.0;

/// A system that handles the 3rd person zoom. Rotation and Position is updated via the
/// shared movement system and visual smoothing.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn camera_movement_system(
    // output
    player_query: Query<(Entity, &ActionState<ClientAction>), With<LocalPlayer>>,
    mut camera_query: Query<(&Camera, &mut Projection, &ChildOf), With<Camera3d>>,
) {
    let Ok((player_entity, action_state)) = player_query.single() else {
        return;
    };

    // handle zoom using leafwing action state
    let zoom_delta = action_state.value(&ClientAction::Zoom);

    if zoom_delta != 0.0 {
        for (camera, mut projection, parent) in camera_query.iter_mut() {
            if camera.is_active
                && parent.get() == player_entity
                && let Projection::Perspective(ref mut perspective) = *projection
            {
                let mut current_fov_deg = perspective.fov.to_degrees();
                current_fov_deg -= zoom_delta;
                current_fov_deg = current_fov_deg.clamp(1.0, 45.0);
                perspective.fov = current_fov_deg.to_radians();
            }
        }
    }
}

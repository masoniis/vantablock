use crate::{input::local_actions::ClientAction, player::LocalPlayerCamera};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use tracing::instrument;

/// Zooms the local player's camera with the scroll wheel. Rotation and position are updated
/// via the shared movement system and visual smoothing.
#[instrument(skip_all)]
pub fn camera_movement_system(
    actions: Single<&ActionState<ClientAction>>,
    mut camera_query: Query<&mut Projection, With<LocalPlayerCamera>>,
) {
    let zoom_delta = actions.value(&ClientAction::Zoom);
    if zoom_delta == 0.0 {
        return;
    }
    for mut projection in camera_query.iter_mut() {
        if let Projection::Perspective(ref mut perspective) = *projection {
            let fov_deg = (perspective.fov.to_degrees() - zoom_delta).clamp(5.0, 90.0);
            perspective.fov = fov_deg.to_radians();
        }
    }
}

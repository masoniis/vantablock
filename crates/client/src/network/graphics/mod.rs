pub mod smoothing;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{
    app::{App, Plugin, PostUpdate},
    prelude::IntoScheduleConfigs,
    transform::TransformSystems,
};

pub struct NetworkGraphicsPlugin;

impl Plugin for NetworkGraphicsPlugin {
    fn build(&self, app: &mut App) {
        // runs after `Update` so the local look from this frame is applied, but before
        // propagation so the camera's `GlobalTransform` is current for rendering
        app.add_systems(
            PostUpdate,
            (
                smoothing::update_logical_position_smoothing,
                smoothing::update_player_look_smoothing,
            )
                .before(TransformSystems::Propagate),
        );
    }
}

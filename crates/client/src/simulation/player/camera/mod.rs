pub mod movement;

pub use movement::*;

// INFO: -----------------------
//         camera plugin
// -----------------------------

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, in_state};
use shared::ecs_core::AppState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<shared::simulation::player::active_camera::ActiveCamera>();

        app.add_systems(
            Update,
            (camera_movement_system, update_camera_chunk_chord_system)
                .chain()
                .run_if(in_state(AppState::Running)),
        );
    }
}

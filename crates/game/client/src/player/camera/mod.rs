pub mod movement;

pub use movement::*;

// INFO: -----------------------
//         camera plugin
// -----------------------------

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, in_state};
use common::lifecycle::state::AppState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            camera_movement_system.run_if(in_state(AppState::Running)),
        );
    }
}

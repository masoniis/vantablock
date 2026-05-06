pub mod movement;

pub use movement::*;

// INFO: -----------------------
//         camera plugin
// -----------------------------

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, in_state};
use shared::lifecycle::state::AppState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (camera_movement_system, sync_player_look_to_server_system)
                .chain()
                .run_if(in_state(AppState::Running)),
        );
    }
}

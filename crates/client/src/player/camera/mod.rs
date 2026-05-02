pub mod movement;
pub mod spawn;

pub use movement::*;
pub use spawn::*;

// INFO: -----------------------
//         camera plugin
// -----------------------------

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Startup, Update, in_state};
use shared::lifecycle::state::enums::AppState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_system);

        app.add_systems(
            Update,
            (camera_movement_system, sync_player_look_to_server_system)
                .chain()
                .run_if(in_state(AppState::Running)),
        );
    }
}

pub mod movement;

pub use movement::*;

// INFO: -----------------------
//         camera plugin
// -----------------------------

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Startup, Update, in_state};
use shared::lifecycle::state::enums::AppState;
use shared::simulation::player::initialize_camera::spawn_camera_system;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_system);

        app.add_systems(
            Update,
            (camera_movement_system, update_camera_chunk_chord_system)
                .chain()
                .run_if(in_state(AppState::Running)),
        );
    }
}

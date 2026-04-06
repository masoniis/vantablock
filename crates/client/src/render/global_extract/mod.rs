pub mod generic_systems;
pub mod resources;

pub use generic_systems::*;
pub use resources::*;

// INFO: ---------------------------
//         Plugin definition
// ---------------------------------

use bevy::app::{App, Plugin};
use bevy::render::ExtractSchedule;

pub struct SimulationExtractionPlugin;

impl Plugin for SimulationExtractionPlugin {
    fn build(&self, app: &mut App) {
        // Extraction here is for global resources used across
        // many different render systems.
        //
        // Anything specific to a pass or otherwise should be
        // located in that pass's dedicated plugin.
        app.add_systems(
            ExtractSchedule,
            (extract_modified_chunk_meshes, extract_active_camera_system),
        );
    }
}

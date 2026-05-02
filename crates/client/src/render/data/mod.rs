pub mod block_meshes;
pub mod camera_phases;
pub mod sun;
pub mod time;

pub use block_meshes::{MeshesToUploadQueue, RenderMeshStorageResource};
pub use sun::ExtractedSun;
pub use time::RenderTimeResource;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::{App, ExtractSchedule, Plugin};
use block_meshes::extract_modified_block_meshes;
use camera_phases::extract_custom_camera_phases_system;

/// A plugin that manages the extraction of simulation-level data to the render world.
pub struct SimulationExtractionPlugin;

impl Plugin for SimulationExtractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            ExtractSchedule,
            (
                extract_modified_block_meshes,
                extract_custom_camera_phases_system,
            ),
        );
    }
}

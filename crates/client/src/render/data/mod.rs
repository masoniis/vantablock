pub mod camera_phases;
pub mod sun;
pub mod time;
pub mod voxel_meshes;

pub use sun::ExtractedSun;
pub use time::RenderTimeResource;
pub use voxel_meshes::{MeshesToUploadQueue, RenderMeshStorageResource};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::{App, ExtractSchedule, Plugin};
use camera_phases::extract_custom_camera_phases_system;
use voxel_meshes::extract_modified_voxel_meshes;

/// A plugin that manages the extraction of simulation-level data to the render world.
pub struct SimulationExtractionPlugin;

impl Plugin for SimulationExtractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            ExtractSchedule,
            (
                extract_modified_voxel_meshes,
                extract_custom_camera_phases_system,
            ),
        );
    }
}

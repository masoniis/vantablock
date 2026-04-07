pub mod camera;
pub mod sun;
pub mod time;
pub mod voxel_meshes;

pub use camera::{RenderCameraResource, extract_active_camera_system};
pub use sun::ExtractedSun;
pub use time::RenderTimeResource;
pub use voxel_meshes::{
    MeshesToUploadQueue, RenderMeshStorageResource, extract_modified_voxel_meshes,
};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin};
use bevy::render::ExtractSchedule;

/// A plugin that manages the extraction of simulation-level data to the render world.
pub struct SimulationExtractionPlugin;

impl Plugin for SimulationExtractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            ExtractSchedule,
            (extract_active_camera_system, extract_modified_voxel_meshes),
        );
    }
}

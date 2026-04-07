pub mod voxel_mesh;

pub use voxel_mesh::VoxelMeshAsset;

// INFO: ----------------------
//         asset plugin
// ----------------------------

use bevy::app::{App, Plugin};
use bevy::asset::AssetApp;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        // register our custom asset types with Bevy's native asset system
        app.init_asset::<VoxelMeshAsset>();
    }
}

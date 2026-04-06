pub mod mesh_asset;

pub use mesh_asset::VoxelChunkMeshAsset;

// INFO: ---------------------------------
//         asset management plugin
// ---------------------------------------

use bevy::app::{App, Plugin};
use bevy::asset::AssetApp;

pub struct AssetManagementPlugin;

impl Plugin for AssetManagementPlugin {
    fn build(&self, app: &mut App) {
        // register our custom asset types with Bevy's native asset system
        app.init_asset::<VoxelChunkMeshAsset>();
    }
}

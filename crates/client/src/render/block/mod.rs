pub mod registry;
pub mod render_data;
pub mod texture_registry;

pub use registry::BlockRenderDataRegistry;
pub use render_data::{BlockFaceTextures, BlockRenderData};
pub use texture_registry::{
    BlockTextureArray, TextureId, TextureLoadError, TextureRegistryResource,
};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin};

pub struct BlockRenderPlugin;

impl Plugin for BlockRenderPlugin {
    fn build(&self, _app: &mut App) {
        // block-specific render setup
    }
}

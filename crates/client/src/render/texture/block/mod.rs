pub mod block_texture_processor;
pub mod block_texture_registry;

pub use block_texture_processor::BlockTextureProcessor;
pub use block_texture_registry::{BlockTextureArray, TextureId, TextureRegistryResource};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{prelude::*, render::texture::block::block_texture_registry::handle_texture_stitching};
use block_texture_registry::LoadTextures;
use shared::{AppStartupLoadingPhase, LoadingAppExt};

pub struct BlockTexturePlugin;

impl Plugin for BlockTexturePlugin {
    fn build(&self, app: &mut App) {
        app.configure_loading_phase::<AppStartupLoadingPhase>()
            .add_node(LoadTextures, handle_texture_stitching);
    }
}

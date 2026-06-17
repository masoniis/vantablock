pub mod registry;
pub mod render_data;

pub use registry::BlockRenderDataRegistry;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::texture::block::block_texture_registry::LoadTextures;
use bevy::prelude::*;
use registry::{LoadRenderRegistry, handle_render_registry};
pub use render_data::{BlockFaceTextures, BlockRenderData};
use shared::lifecycle::load::{LoadBiomes, LoadBlocks};
use shared::{AppStartupLoadingPhase, LoadingAppExt};

pub struct BlockRenderPlugin;

impl Plugin for BlockRenderPlugin {
    fn build(&self, app: &mut App) {
        app.configure_loading_phase::<AppStartupLoadingPhase>()
            .add_node(LoadRenderRegistry, handle_render_registry)
            .add_dependency(LoadRenderRegistry, LoadBlocks)
            .add_dependency(LoadRenderRegistry, LoadTextures)
            .add_dependency(LoadRenderRegistry, LoadBiomes);
    }
}

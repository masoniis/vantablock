pub mod registry;
pub mod render_data;

pub use registry::BlockRenderDataRegistry;
pub use render_data::{BlockFaceTextures, BlockRenderData};

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

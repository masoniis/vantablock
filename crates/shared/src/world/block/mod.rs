pub mod block_definition;
pub mod block_registry;

pub use block_definition::{BlockDescription, load_block_from_str};
pub use block_registry::{AIR_BLOCK_ID, BlockId, BlockRegistry, SOLID_BLOCK_ID};

// INFO: ----------------------
//         block plugin
// ----------------------------

use crate::{
    lifecycle::load::{AppStartupLoadingPhase, LoadBlocks, LoadingAppExt},
    world::block::block_registry::load_block_registry,
};
use bevy::app::{App, Plugin};

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app.configure_loading_phase::<AppStartupLoadingPhase>()
            .add_node(LoadBlocks, load_block_registry);
    }
}

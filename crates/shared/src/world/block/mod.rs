pub mod block_definition;
pub mod block_registry;

pub use block_definition::{BlockDescription, load_block_from_str};
pub use block_registry::{AIR_BLOCK_ID, BlockId, BlockRegistry, SOLID_BLOCK_ID};

// INFO: ----------------------
//         Block plugin
// ----------------------------

use bevy::app::{App, Plugin};

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, _app: &mut App) {
        // Note: Registry initialization is handled asynchronously
        // via the LoadingDag framework to avoid blocking the main thread.
    }
}

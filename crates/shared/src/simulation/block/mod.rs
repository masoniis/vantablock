pub mod block_definition;
pub mod block_registry;
pub mod targeted_block;

pub use block_definition::{BlockDescription, load_block_from_str};
pub use block_registry::{AIR_BLOCK_ID, BlockId, BlockRegistry, SOLID_BLOCK_ID};
pub use targeted_block::TargetedBlock;

// INFO: ----------------------
//         Block plugin
// ----------------------------

use bevy::app::{App, Plugin};

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        // insert resources
        app.init_resource::<TargetedBlock>();
        app.init_resource::<BlockRegistry>();

        // Note: For the client, registry initialization is handled asynchronously
        // to avoid blocking the main thread during startup.
    }
}

pub mod texture_registry;

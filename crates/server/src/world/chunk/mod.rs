pub mod components;
pub mod datagen;
pub mod manager;

pub use components::GeneratedChunkComponentBundle;
pub use datagen::*;
pub use manager::{ServerChunkManager, ServerChunkState};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;

pub struct ServerChunkPlugin;

impl Plugin for ServerChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServerChunkManager>();
    }
}

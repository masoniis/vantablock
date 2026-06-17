pub mod biome;
pub mod block;
pub mod chunk;

// INFO: ----------------------
//         plugin group
// ----------------------------

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};
use biome::BiomePlugin;
use block::BlockPlugin;
use chunk::ChunkPlugin;

/// A plugin group containing shared simulation and game logic plugins used by both client and server.
pub struct WorldPlugins;

impl PluginGroup for WorldPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ChunkPlugin)
            .add(BlockPlugin)
            .add(BiomePlugin)
    }
}

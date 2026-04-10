pub mod load;
pub mod network;
pub mod prelude;
pub mod simulation;
pub mod state;

pub use prelude::*;

// INFO: -----------------------------
//         shared plugin group
// -----------------------------------

use bevy::app::PluginGroupBuilder;
use bevy::prelude::PluginGroup;

/// A plugin group containing shared simulation and game logic plugins used by both client and server.
pub struct SharedPlugins;

impl PluginGroup for SharedPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(state::SimulationLifecyclePlugin)
            .add(network::NetworkPlugin)
            .add(simulation::asset::AssetPlugin)
            .add(simulation::biome::BiomePlugin)
            .add(simulation::block::BlockPlugin)
            .add(simulation::chunk::ChunkLoadingPlugin)
            .add(simulation::terrain::TerrainGenerationPlugin)
            .add(simulation::time::TimeControlPlugin)
    }
}

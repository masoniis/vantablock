//! # The Vantablock Server Library

pub mod network;
pub mod prelude;
pub mod simulation;
pub mod state;

pub use prelude::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

/// Server-side simulation and orchestration plugins.
pub struct ServerPlugins;

impl PluginGroup for ServerPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(network::ServerNetworkPlugin)
            .add(simulation::ServerSimulationPlugin)
    }
}

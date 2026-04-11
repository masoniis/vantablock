pub mod network;
pub mod simulation;
pub mod state;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::PluginGroup;

/// Server-side simulation and orchestration plugins.
pub struct ServerPlugins;

impl PluginGroup for ServerPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(network::ServerNetworkPlugin)
            .add(simulation::ServerSimulationPlugin)
    }
}

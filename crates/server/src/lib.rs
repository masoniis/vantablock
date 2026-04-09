pub mod state;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::PluginGroup;
use shared::SharedPlugins;

/// Server-side simulation and orchestration plugins.
pub struct ServerPlugins;

impl PluginGroup for ServerPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add_group(SharedPlugins)
    }
}

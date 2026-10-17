pub mod load;
pub mod state;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::PluginGroupBuilder;
use bevy::prelude::PluginGroup;

/// A plugin group containing shared lifecycle stuff.
pub struct ServerLifecyclePlugins;

impl PluginGroup for ServerLifecyclePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(state::StatePlugin)
            .add(load::ServerLoadPlugin)
    }
}

pub mod load;
pub mod state;

pub use load::*;
pub use state::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::PluginGroupBuilder;
use bevy::prelude::PluginGroup;

/// A plugin group containing client lifecycle stuff.
pub struct LifecyclePlugins;

impl PluginGroup for LifecyclePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ClientStatePlugin)
            .add(ClientLoadPlugin)
    }
}

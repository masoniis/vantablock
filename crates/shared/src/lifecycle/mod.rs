pub mod load;
pub mod state;

pub use load::*;
pub use state::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::PluginGroupBuilder;
use bevy::prelude::PluginGroup;

/// A plugin group containing shared lifecycle stuff.
pub struct SharedLifecyclePlugins;

impl PluginGroup for SharedLifecyclePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(load::LoadPlugin)
            .add(state::StatePlugin)
    }
}

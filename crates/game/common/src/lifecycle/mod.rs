pub mod load;
mod paths;
pub mod scheduling;
pub mod state;

pub use load::*;
pub use paths::{PathsPlugin, PersistentPathsResource};
pub use scheduling::*;
pub use state::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

/// A plugin group containing shared lifecycle stuff.
pub struct SharedLifecyclePlugins;

impl PluginGroup for SharedLifecyclePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(paths::PathsPlugin)
            .add(scheduling::SharedSchedulingPlugin)
            .add(state::StatePlugin)
            .add(load::StartupLoadPlugin)
    }
}

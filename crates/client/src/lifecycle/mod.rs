pub mod load;
pub mod scheduling;
pub mod state;

pub use load::*;
pub use state::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};
use scheduling::ClientSchedulingPlugin;

/// A plugin group containing client lifecycle stuff.
pub struct LifecyclePlugins;

impl PluginGroup for LifecyclePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ClientLoadPlugin)
            .add(ClientSchedulingPlugin)
            .add(ClientStatePlugin)
    }
}

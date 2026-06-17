//! # The Vantablock Shared Library

pub mod lifecycle;
pub mod network;
pub mod player;
pub mod prelude;
pub mod time;
pub mod world;

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
            .add_group(lifecycle::SharedLifecyclePlugins)
            .add_group(world::WorldPlugins)
            .add(time::TimeControlPlugin)
            .add(player::SharedPlayerPlugin)
    }
}

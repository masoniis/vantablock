//! # The Vantablock Shared Library

pub mod events;
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
use bevy::ecs::message::Messages;
use bevy::prelude::{App, Plugin, PluginGroup};

/// A plugin group containing shared simulation and game logic plugins used by both client and server.
pub struct SharedPlugins;

impl PluginGroup for SharedPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(SharedEventsPlugin)
            .add_group(lifecycle::SharedLifecyclePlugins)
            .add(world::biome::BiomePlugin)
            .add(world::block::BlockPlugin)
            .add(time::TimeControlPlugin)
            .add(player::SharedPlayerPlugin)
    }
}

struct SharedEventsPlugin;

impl Plugin for SharedEventsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Messages<events::RequestSingleplayerSession>>();
    }
}

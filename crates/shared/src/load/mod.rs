// Async loading is a module for handling everything related to the completion of asynchronous tasks.
//
// This logic ties into the state machine, enabling generic state transitions based on when a set of
// tasks complete.

pub mod components;
pub mod resources;
pub mod systems;

pub use components::*;
pub use resources::*;
pub use systems::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

// TODO: more robust loading setup, also add other loading systems to the plugin

use bevy::prelude::{App, Plugin};

pub struct LoadPlugin;

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingTracker>();
    }
}

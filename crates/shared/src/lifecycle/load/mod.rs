// Async loading is a module for handling everything related to the completion of asynchronous tasks.
//
// This logic ties into the state machine, enabling generic state transitions based on when a set of
// tasks complete.

pub mod components;
pub mod systems;

pub use components::*;
pub use systems::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::{
    load::loading_tasks::loading_is_complete, state::enums::AppState, state::transition_to,
};
use bevy::prelude::*;

pub struct LoadPlugin;

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        // polling systems and tracking load state for app startup
        app.add_systems(
            Update,
            (
                poll_tasks::<AppStartupLoadingPhase>,
                transition_to(AppState::Running)
                    .run_if(loading_is_complete::<AppStartupLoadingPhase>),
            )
                .chain()
                .run_if(in_state(AppState::StartingUp)),
        );
    }
}

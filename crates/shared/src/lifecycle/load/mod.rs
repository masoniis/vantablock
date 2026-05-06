// A module for handling everything related to the completion of asynchronous tasks.
//
// This logic ties into the state machine, enabling generic state transitions based on when a set of
// tasks complete.

mod dag;
mod phases;
mod tasks;

pub use dag::*;
pub use phases::*;
pub use tasks::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{lifecycle::state::AppState, transition_to};
use bevy::prelude::*;

/// A plugin that manages the transition from `AppState::StartingUp` to `AppState::Running`
/// based on the completion of the `AppStartupLoadingPhase` DAG.
pub struct StartupLoadPlugin;

impl Plugin for StartupLoadPlugin {
    fn build(&self, app: &mut App) {
        // kickoff the app startup loading phase when starting up
        app.add_systems(
            OnEnter(AppState::StartingUp),
            kickoff_loading_phase::<AppStartupLoadingPhase>,
        );

        // handle transition to running state when app startup is done
        app.add_systems(
            Update,
            (transition_to(AppState::Running))
                .run_if(loading_dag_is_complete::<AppStartupLoadingPhase>)
                .run_if(in_state(AppState::StartingUp)),
        );

        // cleanup when leaving starting up state
        app.add_systems(
            OnExit(AppState::StartingUp),
            nuke_loading_dag::<AppStartupLoadingPhase>,
        );
    }
}

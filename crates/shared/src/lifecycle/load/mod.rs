// Async loading is a module for handling everything related to the completion of asynchronous tasks.
//
// This logic ties into the state machine, enabling generic state transitions based on when a set of
// tasks complete.

pub mod components;
mod systems;

pub use components::*;
pub use systems::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::state::{SimulationState, enums::AppState, transition_to};
use bevy::prelude::*;

pub struct LoadPlugin;

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        // TODO: remove temp fake work system for testing
        app.add_systems(OnEnter(SimulationState::Loading), start_fake_work_system);

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
        )
        .add_systems(
            OnExit(AppState::StartingUp),
            cleanup_orphaned_tasks::<AppStartupLoadingPhase>,
        );

        // polling systems and tracking load state for simulation loading
        app.add_systems(
            Update,
            (
                poll_tasks::<SimulationLoadingPhase>,
                transition_to(SimulationState::Running)
                    .run_if(loading_is_complete::<SimulationLoadingPhase>),
            )
                .chain()
                .run_if(in_state(SimulationState::Loading)),
        )
        .add_systems(
            OnExit(SimulationState::Loading),
            cleanup_orphaned_tasks::<SimulationLoadingPhase>,
        );
    }
}

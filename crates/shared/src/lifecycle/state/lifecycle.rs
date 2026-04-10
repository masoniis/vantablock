use crate::lifecycle::load::{
    check_loading_complete, cleanup_orphaned_tasks, poll_tasks, start_fake_work_system,
    SimulationLoadingPhase,
};
use crate::lifecycle::state::SimulationState;
use bevy::prelude::{
    in_state, App, AppExtStates, IntoScheduleConfigs, OnEnter, OnExit, Plugin, Update,
};

pub struct SimulationLifecyclePlugin;

/// A plugin for the simulation world that sets up the necessary
/// systems for handling the application lifecycle. This primarily
/// involves orchestration of loading tasks and state transitions.
impl Plugin for SimulationLifecyclePlugin {
    fn build(&self, app: &mut App) {
        // INFO: -----------------------
        //         async loading
        // -----------------------------

        // load cleanup to run after transitions
        app.add_systems(
            OnExit(SimulationState::Loading),
            cleanup_orphaned_tasks::<SimulationLoadingPhase>,
        );

        // systems to ensure rigidity
        app.add_systems(OnEnter(SimulationState::Loading), start_fake_work_system);
    }
}

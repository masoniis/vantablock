use crate::lifecycle::load::{
    SimulationPhase, check_loading_complete, cleanup_orphaned_tasks, poll_tasks,
    start_fake_work_system,
};
use crate::lifecycle::state::SimulationState;
use bevy::prelude::{
    App, AppExtStates, IntoScheduleConfigs, OnEnter, OnExit, Plugin, Update, in_state,
};

pub struct SimulationLifecyclePlugin;

/// A plugin for the simulation world that sets up the necessary
/// systems for handling the application lifecycle. This primarily
/// involves orchestration of loading tasks and state transitions.
impl Plugin for SimulationLifecyclePlugin {
    fn build(&self, app: &mut App) {
        // INFO: ---------------------------
        //         state transitions
        // ---------------------------------
        app.init_state::<SimulationState>();

        // INFO: -----------------------
        //         async loading
        // -----------------------------

        // polling systems and tracking load state
        app.add_systems(
            Update,
            (
                poll_tasks::<SimulationPhase>,
                check_loading_complete::<SimulationPhase, SimulationState>(SimulationState::Running)
                    .after(poll_tasks::<SimulationPhase>),
            )
                .run_if(in_state(SimulationState::Loading)),
        );

        // load cleanup to run after transitions
        app.add_systems(
            OnExit(SimulationState::Loading),
            cleanup_orphaned_tasks::<SimulationPhase>,
        );

        // systems to ensure rigidity
        app.add_systems(OnEnter(SimulationState::Loading), start_fake_work_system);
    }
}

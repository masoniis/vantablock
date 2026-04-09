use crate::load::{
    OnLoadComplete, master_finalize_loading_system, poll_simulation_loading_tasks,
    reset_loading_tracker_system, start_fake_work_system,
};
use crate::state::SimulationState;
use bevy::prelude::{
    App, AppExtStates, IntoScheduleConfigs, OnExit, Plugin, Startup, Update, in_state,
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

        // polling systems and tracking load state
        app.add_systems(
            Update,
            (poll_simulation_loading_tasks.run_if(in_state(SimulationState::Loading)),),
        );

        // load cleanup to run after transitions
        app.add_systems(
            OnExit(SimulationState::Loading),
            reset_loading_tracker_system,
        );

        // systems to ensure rigidity
        app.add_systems(Startup, start_fake_work_system);

        // INFO: ---------------------------
        //         state transitions
        // ---------------------------------
        app.init_state::<SimulationState>();

        app.add_systems(
            Update,
            (master_finalize_loading_system::<SimulationState>,)
                .run_if(in_state(SimulationState::Loading)),
        );

        // initial startup loading state should take us from loading
        // to running/playing once they finish.
        app.insert_resource(OnLoadComplete::new(SimulationState::Running));
    }
}

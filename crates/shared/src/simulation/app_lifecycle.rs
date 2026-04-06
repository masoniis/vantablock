use crate::ecs_core::{
    AppState, GameState,
    async_loading::{
        OnLoadComplete, master_finalize_loading_system, poll_simulation_loading_tasks,
        reset_loading_tracker_system, start_fake_work_system,
    },
};
use bevy::prelude::{
    App, AppExtStates, IntoScheduleConfigs, OnExit, Plugin, Startup, Update, in_state,
};

pub struct AppLifecyclePlugin;

/// A plugin for the simulation world that sets up the necessary
/// systems for handling the application lifecycle. This primarily
/// involves orchestration of loading tasks and state transitions.
impl Plugin for AppLifecyclePlugin {
    fn build(&self, app: &mut App) {
        // INFO: -----------------------
        //         async loading
        // -----------------------------

        // polling systems and tracking load state
        app.add_systems(
            Update,
            (poll_simulation_loading_tasks.run_if(in_state(AppState::StartingUp)),),
        );

        // load cleanup to run after transitions
        app.add_systems(OnExit(AppState::StartingUp), reset_loading_tracker_system);

        // systems to ensure rigidity
        app.add_systems(Startup, start_fake_work_system);

        // INFO: ---------------------------
        //         state transitions
        // ---------------------------------
        app.init_state::<AppState>();
        app.init_state::<GameState>();

        app.add_systems(
            Update,
            (
                master_finalize_loading_system::<AppState>,
                master_finalize_loading_system::<GameState>,
            )
                .run_if(in_state(AppState::StartingUp)),
        );

        // initial startup loading state should take us from loading
        // to running/playing once they finish.
        app.insert_resource(OnLoadComplete::new(AppState::Running))
            .insert_resource(OnLoadComplete::new(GameState::Playing));
    }
}

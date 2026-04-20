pub mod enums;

pub use enums::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::{
    prelude::{App, Plugin},
    state::app::AppExtStates,
};
use shared::transition_to;
use shared::{
    FixedUpdateSet,
    lifecycle::load::{AppStartupLoadingPhase, cleanup_orphaned_tasks},
    lifecycle::state::{SimulationState, enums::AppState},
};

pub struct ClientStatePlugin;

impl Plugin for ClientStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<ClientState>();
        app.add_sub_state::<InGameState>();

        // INFO: -----------------------
        //         async loading
        // -----------------------------

        // polling systems for simulation-linked client state transitions
        app.add_systems(
            Update,
            (
                // transition from loading to main menu once simulation is ready
                transition_to(ClientState::MainMenu)
                    .run_if(in_state(ClientState::Loading))
                    .run_if(in_state(SimulationState::Running)),
            )
                .run_if(in_state(AppState::Running)),
        );

        // load cleanup to run after transitions
        app.add_systems(
            OnExit(AppState::StartingUp),
            cleanup_orphaned_tasks::<AppStartupLoadingPhase>,
        );

        // configure system sets to be state-bound
        app.configure_sets(
            FixedUpdate,
            (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic)
                .run_if(in_state(ClientState::InGame)),
        );

        // INFO: ---------------------------
        //         state transitions
        // ---------------------------------

        app.add_systems(
            OnExit(AppState::StartingUp),
            |mut window: Query<&mut Window, With<PrimaryWindow>>| {
                if let Ok(mut win) = window.single_mut() {
                    win.visible = true;
                }
            },
        );
    }
}

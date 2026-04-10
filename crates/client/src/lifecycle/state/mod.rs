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
use shared::{
    FixedUpdateSet,
    lifecycle::load::{AppStartupLoadingPhase, LoadingTaskComponent, cleanup_orphaned_tasks},
    lifecycle::state::enums::AppState,
};
use shared::{loading_is_complete, transition_to};

pub struct ClientStatePlugin;

impl Plugin for ClientStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<ClientGameState>();

        // INFO: -----------------------
        //         async loading
        // -----------------------------

        // polling systems for simulation-linked client state transitions
        app.add_systems(
            Update,
            (transition_to(ClientGameState::Playing)
                .run_if(loading_is_complete::<LoadingTaskComponent>)
                .run_if(in_state(ClientGameState::MainMenu)),)
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
            (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).run_if(
                in_state(ClientGameState::Playing).or(in_state(ClientGameState::Connecting)),
            ),
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

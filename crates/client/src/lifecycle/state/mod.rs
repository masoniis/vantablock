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
    lifecycle::load::{
        check_loading_complete, cleanup_orphaned_tasks, AppStartupLoadingPhase,
        LoadingTaskComponent,
    },
    lifecycle::state::enums::AppState,
    {FixedUpdateSet, RenderPrepSet},
};

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
            (
                check_loading_complete::<LoadingTaskComponent, ClientGameState>(
                    ClientGameState::Playing,
                )
                .run_if(in_state(ClientGameState::MainMenu)),
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
            (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).run_if(
                in_state(ClientGameState::Playing).or(in_state(ClientGameState::Connecting)),
            ),
        );

        app.configure_sets(
            PostUpdate,
            RenderPrepSet.run_if(
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

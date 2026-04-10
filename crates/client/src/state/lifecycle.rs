use crate::state::enums::ClientGameState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shared::lifecycle::load::{
    check_loading_complete, cleanup_orphaned_tasks, poll_tasks, AppStartupLoadingPhase,
    LoadingTaskComponent,
};
use shared::lifecycle::state::enums::AppState;
use shared::{FixedUpdateSet, RenderPrepSet};

pub struct ClientLifecyclePlugin;

impl Plugin for ClientLifecyclePlugin {
    fn build(&self, app: &mut App) {
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

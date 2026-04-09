use crate::state::enums::{ClientAppState, ClientGameState};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shared::{
    FixedUpdateSet, RenderPrepSet,
    load::{
        LoadingTracker, OnLoadComplete, master_finalize_loading_system,
        reset_loading_tracker_system,
    },
};

pub struct ClientLifecyclePlugin;

impl Plugin for ClientLifecyclePlugin {
    fn build(&self, app: &mut App) {
        // load cleanup to run after transitions
        app.add_systems(
            OnExit(ClientAppState::StartingUp),
            reset_loading_tracker_system,
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
        app.init_state::<ClientAppState>()
            .add_sub_state::<ClientGameState>();

        app.add_systems(
            Update,
            (
                master_finalize_loading_system::<ClientAppState>,
                master_finalize_loading_system::<ClientGameState>,
                show_window_when_ready,
            )
                .run_if(in_state(ClientAppState::StartingUp)),
        );

        // initial startup loading state should take us from loading
        // to running/playing once they finish.
        app.insert_resource(OnLoadComplete::new(ClientAppState::Running))
            .insert_resource(OnLoadComplete::new(ClientGameState::Playing));
    }
}

/// A system that makes the window visible once the loading tracker reports readiness.
fn show_window_when_ready(
    mut query: Query<&mut Window, With<PrimaryWindow>>,
    loading_tracker: Res<LoadingTracker>,
) {
    if loading_tracker.is_ready()
        && let Ok(mut window) = query.single_mut()
    {
        window.visible = true;
    }
}

mod enums;

pub use enums::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{prelude::*, state::app::AppExtStates, window::PrimaryWindow};
use shared::{FixedUpdateSet, lifecycle::state::AppState};

pub struct ClientStatePlugin;

impl Plugin for ClientStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<ClientLifecycleState>()
            .add_sub_state::<InGameState>()
            .add_sub_state::<SessionTopology>();

        // INFO: -----------------------
        //         async loading
        // -----------------------------

        // load cleanup to run after transitions
        // (handled automatically by Bevy despawn_recursive now)

        // configure system sets to be state-bound
        app.configure_sets(
            FixedUpdate,
            (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic)
                .run_if(in_state(ClientLifecycleState::InGame)),
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

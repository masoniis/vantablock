use crate::{
    lifecycle::ClientLifecycleState,
    ui::{screens::status_card, systems::spawn_menu_camera_system, theme::backdrop},
};
use bevy::prelude::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

pub struct LaunchingClientScreenPlugin;

impl Plugin for LaunchingClientScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ClientLifecycleState::Launching),
            (spawn_menu_camera_system, spawn_launching_client_screen),
        );
    }
}

// INFO: ------------------------------
//         interface definition
// ------------------------------------

/// A UI intended to appear for the duration of the `ClientLifecycleState::Launching` state.
fn spawn_launching_client_screen(mut commands: Commands) {
    info!("Spawning Launching UI...");
    commands
        .spawn_scene(bsn! {
            backdrop()
            Children [ status_card("Launching", "Baking assets and building the geometry library") ]
        })
        .insert(DespawnOnExit(ClientLifecycleState::Launching));
}

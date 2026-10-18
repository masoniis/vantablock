//! The client's view of the shared world.

pub mod clock;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::ClientLifecycleState;
use bevy::prelude::*;
use shared::time::advance_world_clock_system;

pub struct ClientWorldPlugin;

impl Plugin for ClientWorldPlugin {
    fn build(&self, app: &mut App) {
        // advance every frame for smooth animation, then snap to the server's syncs
        app.add_systems(
            Update,
            (advance_world_clock_system, clock::apply_clock_sync_system)
                .chain()
                .run_if(in_state(ClientLifecycleState::InGame)),
        );
    }
}

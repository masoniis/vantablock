pub mod clock;
pub mod scene;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::state::ServerState;
use bevy::prelude::*;
use shared::time::advance_world_clock_system;

/// The server's authoritative world: the clock and the objects placed in it.
pub(crate) struct ServerWorldPlugin;

impl Plugin for ServerWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, advance_world_clock_system)
            .add_systems(
                Update,
                (
                    clock::handle_time_jump_requests,
                    clock::handle_day_cycle_toggle_requests,
                ),
            )
            .add_systems(OnEnter(ServerState::Running), scene::spawn_scene_system);
    }
}

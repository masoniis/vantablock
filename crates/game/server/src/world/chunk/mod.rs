pub mod chunk_map;
pub mod components;

pub use chunk_map::ChunkMap;
pub use components::GeneratedChunkComponentBundle;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

mod tasks;

use crate::lifecycle::state::ServerState;
use bevy::prelude::*;

pub(crate) struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>();

        // chunk task generation logic
        app.add_systems(
            FixedUpdate,
            (
                tasks::start_pending_generation_tasks_system,
                tasks::poll_chunk_generation_tasks,
            )
                .run_if(in_state(ServerState::Running)),
        );
    }
}

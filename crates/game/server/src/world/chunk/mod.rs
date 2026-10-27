pub mod chunk_map;
pub mod components;
pub mod loading;

pub use chunk_map::ChunkMap;
pub use components::GeneratedChunkComponentBundle;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

mod tasks;

use crate::lifecycle::state::ServerState;
use bevy::prelude::*;
use loading::{
    manage_player_chunk_loading_system, queue_client_chunk_requests_system,
    sync_chunk_data_to_clients_system, unload_distant_chunks_system,
};

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

        // chunk streaming to clients
        app.add_systems(
            Update,
            (
                (
                    unload_distant_chunks_system,
                    manage_player_chunk_loading_system,
                )
                    .chain(),
                (
                    queue_client_chunk_requests_system,
                    sync_chunk_data_to_clients_system,
                )
                    .chain(),
            )
                .run_if(in_state(ServerState::Running)),
        );
    }
}

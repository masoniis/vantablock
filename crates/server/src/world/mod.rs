pub mod chunk;
pub mod chunk_loading;
pub mod terrain;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::state::ServerState;
use bevy::prelude::*;
use chunk_loading::{
    manage_player_chunk_loading_system, queue_client_chunk_requests_system,
    sync_chunk_data_to_clients_system, unload_distant_chunks_system,
};
use terrain::TerrainGenerationPlugin;

pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((TerrainGenerationPlugin, chunk::ChunkPlugin));

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

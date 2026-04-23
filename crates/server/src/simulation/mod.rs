// INFO: ---------------------------
//         plugin definition
// ---------------------------------

pub mod chunk;
pub mod chunk_loading;
pub mod terrain;

#[cfg(test)]
mod chunk_loading_test;

use bevy::prelude::*;
use chunk::{poll_chunk_generation_tasks, start_pending_generation_tasks_system};
use chunk_loading::{manage_player_chunk_loading_system, sync_chunk_data_to_clients_system};
use shared::network::state::NetworkingMode;
use terrain::TerrainGenerationPlugin;

pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainGenerationPlugin);

        app.add_systems(
            Update,
            (
                manage_player_chunk_loading_system,
                sync_chunk_data_to_clients_system,
            )
                .run_if(in_state(NetworkingMode::Internal)),
        );

        app.add_systems(
            FixedUpdate,
            (
                start_pending_generation_tasks_system,
                poll_chunk_generation_tasks,
            )
                .run_if(in_state(NetworkingMode::Internal)),
        );
    }
}

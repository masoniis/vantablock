// INFO: ---------------------------
//         plugin definition
// ---------------------------------

pub mod chunk_loading;

use bevy::prelude::*;
use chunk_loading::{
    manage_player_chunk_loading_system, send_welcome_system, sync_chunk_data_to_clients_system,
};
use shared::network::state::NetworkingMode;

pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                send_welcome_system,
                manage_player_chunk_loading_system,
                sync_chunk_data_to_clients_system,
            )
                .run_if(in_state(NetworkingMode::Internal)),
        );
    }
}

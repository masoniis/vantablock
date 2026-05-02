pub mod chunk_data;
pub mod demultiplex;
pub mod ecs_messages;
pub mod handle_welcome;

pub use ecs_messages::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct NetworkReceivePlugin;

impl Plugin for NetworkReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WelcomeEvent>()
            .add_message::<ReceivedChunkDataEvent>();

        app.add_systems(
            Update,
            (
                chunk_data::apply_received_chunk_data_system,
                demultiplex::translate_server_network_messages,
                handle_welcome::handle_welcome_system,
            ),
        );
    }
}

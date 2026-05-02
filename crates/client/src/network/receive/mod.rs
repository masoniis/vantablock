pub mod demultiplex;
pub mod ecs_messages;

pub use ecs_messages::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct NetworkReceivePlugin;

impl Plugin for NetworkReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WelcomeMessage>()
            .add_message::<ReceivedCompressedChunkMessage>();

        app.add_systems(Update, demultiplex::translate_server_network_messages);
    }
}

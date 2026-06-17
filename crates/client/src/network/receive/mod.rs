mod demultiplex;
mod ecs_messages;

pub use ecs_messages::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct NetworkReceivePlugin;

impl Plugin for NetworkReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InboundCompressedChunkMessage>();

        app.add_systems(Update, demultiplex::translate_server_network_messages);
    }
}

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::network::messages::{ReceivedChunkDataEvent, WelcomeEvent};
use bevy::ecs::message::{MessageWriter, Messages};
use bevy::prelude::*;
use lightyear::prelude::MessageReceiver;
use shared::network::protocol::server::ServerMessage;

pub struct ClientMessageHandlerPlugin;

impl Plugin for ClientMessageHandlerPlugin {
    fn build(&self, app: &mut App) {
        // Register the messages so Bevy knows about them
        app.init_resource::<Messages<WelcomeEvent>>()
            .init_resource::<Messages<ReceivedChunkDataEvent>>();

        app.add_systems(Update, translate_server_messages);
    }
}

pub fn translate_server_messages(
    mut query: Query<&mut MessageReceiver<ServerMessage>>,
    mut ev_welcome: MessageWriter<WelcomeEvent>,
    mut ev_chunk: MessageWriter<ReceivedChunkDataEvent>,
) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
            match message {
                ServerMessage::Welcome { spawn_pos, .. } => {
                    ev_welcome.write(WelcomeEvent { spawn_pos });
                }
                ServerMessage::ChunkData { coord, data } => {
                    ev_chunk.write(ReceivedChunkDataEvent { coord, data });
                }
                _ => {
                    warn!("Unhandled message received");
                }
            }
        }
    }
}

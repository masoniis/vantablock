use bevy::prelude::*;
use shared::network::protocol::server::ServerMessage;

pub struct ClientMessageHandlerPlugin;

impl Plugin for ClientMessageHandlerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_server_messages);
    }
}

fn handle_server_messages(mut messages: MessageReader<ServerMessage>) {
    for message in messages.read() {
        match message {
            ServerMessage::Welcome {
                entity_id: _,
                spawn_pos,
            } => {
                info!("Welcome message received! Spawn pos: {:?}", spawn_pos);
            }
            ServerMessage::ChunkData { coord, data: _ } => {
                trace!("Received chunk data for {:?}", coord);
            }
            _ => {
                warn!("Unhandled message recieved: {:?}", message);
            }
        }
    }
}

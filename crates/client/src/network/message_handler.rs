use bevy::prelude::*;
use lightyear::prelude::MessageReceiver;
use shared::network::protocol::server::ServerMessage;

pub struct ClientMessageHandlerPlugin;

impl Plugin for ClientMessageHandlerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_server_messages);
    }
}

fn handle_server_messages(mut query: Query<&mut MessageReceiver<ServerMessage>>) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
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
                    warn!("Unhandled message received");
                }
            }
        }
    }
}

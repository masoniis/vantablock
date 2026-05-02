use crate::network::ecs_messages::{ReceivedCompressedChunkMessage, WelcomeMessage};
use bevy::{ecs::message::MessageWriter, prelude::*};
use lightyear::prelude::MessageReceiver;
use shared::network::protocol::ServerMessage;

/// Network demultiplexer, translates a `ServerMessage` into a local ecs message
/// to be consumed directly by other systems.
///
/// While it produces a decent amount of boiler plate since we need a client
/// ECS event for each enum variant of the incoming server message, this is
/// worth for efficiency (iterating demuiltiplexed data).
pub fn translate_server_network_messages(
    // incoming network messages
    mut query: Query<&mut MessageReceiver<ServerMessage>>,
    // outgoing ECS messages
    mut ev_welcome: MessageWriter<WelcomeMessage>,
    mut ev_chunk: MessageWriter<ReceivedCompressedChunkMessage>,
) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
            match message {
                ServerMessage::Welcome { spawn_pos, .. } => {
                    ev_welcome.write(WelcomeMessage { spawn_pos });
                }
                ServerMessage::ChunkData { coord, data } => {
                    ev_chunk.write(ReceivedCompressedChunkMessage { coord, data });
                }
                ServerMessage::SyncTime { game_time, tick } => {
                    info!("SyncTime received: game_time={}, tick={}", game_time, tick);
                }
                _ => {
                    warn!("Unhandled message received: {:?}", message);
                }
            }
        }
    }
}

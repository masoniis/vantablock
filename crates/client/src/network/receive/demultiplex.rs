use crate::network::ecs_messages::{ReceivedChunkDataEvent, WelcomeEvent};
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
                    // decompress the data using zstd
                    // TODO: compression should happen async and out of the demultiplexer
                    match zstd::decode_all(&data[..]) {
                        Ok(decompressed) => {
                            trace!(target:"client_network", "Decompressed chunk {:?} ({} -> {} bytes)", coord, data.len(), decompressed.len());
                            ev_chunk.write(ReceivedChunkDataEvent {
                                coord,
                                data: decompressed,
                            });
                        }
                        Err(e) => {
                            error!("Failed to decompress chunk data for {:?}: {}", coord, e);
                        }
                    }
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

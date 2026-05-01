// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::network::messages::{ReceivedChunkDataEvent, WelcomeEvent};
use bevy::{
    ecs::message::{MessageWriter, Messages},
    prelude::*,
};
use lightyear::prelude::MessageReceiver;
use shared::network::protocol::ServerMessage;

pub struct ClientMessageHandlerPlugin;

impl Plugin for ClientMessageHandlerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Messages<WelcomeEvent>>()
            .init_resource::<Messages<ReceivedChunkDataEvent>>();

        app.add_systems(
            Update,
            (
                translate_server_messages,
                super::handle_welcome::handle_welcome_system,
            ),
        );
    }
}

fn translate_server_messages(
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
                    // decompress the data using zstd
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

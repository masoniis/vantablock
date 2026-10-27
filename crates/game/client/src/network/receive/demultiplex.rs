use crate::network::{InboundBlockUpdateMessage, InboundCompressedChunkMessage};
use bevy::{ecs::message::MessageWriter, prelude::*};
use common::network::protocol::ServerMessage;
use lightyear::prelude::MessageReceiver;

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
    mut ev_chunk: MessageWriter<InboundCompressedChunkMessage>,
    mut ev_block_update: MessageWriter<InboundBlockUpdateMessage>,
) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
            match message {
                ServerMessage::ChunkData { coord, data } => {
                    ev_chunk.write(InboundCompressedChunkMessage { coord, data });
                }
                ServerMessage::BlockUpdate { position, block_id } => {
                    ev_block_update.write(InboundBlockUpdateMessage { position, block_id });
                }
                ServerMessage::SyncTime { game_time, tick } => {
                    debug!("SyncTime received: game_time={}, tick={}", game_time, tick);
                }
            }
        }
    }
}

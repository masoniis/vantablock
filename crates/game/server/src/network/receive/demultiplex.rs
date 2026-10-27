use crate::network::receive::ecs_messages::*;
use bevy::{ecs::message::MessageWriter, prelude::*};
use common::network::protocol::ClientMessage;
use lightyear::prelude::MessageReceiver;

/// Network demultiplexer, translates a `ClientMessage` into a local ecs message
///
/// This system listens for incoming client messages and dispatches a corresponding
/// ECS message for each enum variant of the incoming client message. This is
/// necessary to allow multiple systems to react to specific client messages.
pub fn translate_client_network_messages(
    // incoming network messages
    mut query: Query<(Entity, &mut MessageReceiver<ClientMessage>)>,
    // outgoing ECS messages
    mut ev_request_chunk: MessageWriter<InboundRequestChunkMessage>,
    mut ev_break_block: MessageWriter<InboundBreakBlockMessage>,
    mut ev_place_block: MessageWriter<InboundPlaceBlockMessage>,
    mut ev_teleport: MessageWriter<InboundTeleportMessage>,
) {
    for (player, mut receiver) in query.iter_mut() {
        for message in receiver.receive() {
            match message {
                ClientMessage::RequestChunks(coords) => {
                    for coord in coords {
                        ev_request_chunk.write(InboundRequestChunkMessage { player, coord });
                    }
                }
                ClientMessage::BreakBlock { position } => {
                    ev_break_block.write(InboundBreakBlockMessage { player, position });
                }
                ClientMessage::PlaceBlock { position, block_id } => {
                    ev_place_block.write(InboundPlaceBlockMessage {
                        player,
                        position,
                        block_id,
                    });
                }
                ClientMessage::Teleport { position } => {
                    ev_teleport.write(InboundTeleportMessage { player, position });
                }
                ClientMessage::RequestTimeJump { amount: _ } => {
                    // TODO: implement time jump handling
                    warn!("RequestTimeJump received but not yet implemented");
                }
            }
        }
    }
}

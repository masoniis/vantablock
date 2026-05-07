//! Defines local ECS messages for demultiplexed network messages received from clients.
//! These messages are dispatched by the network demultiplexer to allow multiple systems
//! to react to specific client messages without all of them needing to read from the
//! `MessageReceiver<ClientMessage>` directly.

use bevy::ecs::message::Message;
use shared::world::chunk::ChunkCoord;
use bevy::prelude::*;

#[derive(Message, Clone)]
pub struct InboundRequestChunkMessage {
    pub player: Entity,
    pub coord: ChunkCoord,
}

#[derive(Message, Clone)]
pub struct InboundBreakBlockMessage {
    pub player: Entity,
    pub position: IVec3,
}

#[derive(Message, Clone)]
pub struct InboundPlaceBlockMessage {
    pub player: Entity,
    pub position: IVec3,
    pub block_id: u8,
}

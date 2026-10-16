//! Defines the local ecs messages for demultiplexed network messages
//!
//! For the most part, these messages are simply duplicated structurs
//! of the original network messages our client receives.

use bevy::ecs::message::Message;
use bevy::prelude::*;
use shared::world::chunk::ChunkCoord;

#[derive(Message)]
pub struct InboundCompressedChunkMessage {
    pub coord: ChunkCoord,
    pub data: Vec<u8>,
}

#[derive(Message)]
pub struct InboundBlockUpdateMessage {
    pub position: IVec3,
    pub block_id: u8,
}

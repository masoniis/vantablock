//! Defines the local ecs messages for demultiplexed network messages
//!
//! For the most part, these messages are simply duplicated structurs
//! of the original network messages our client receives.

use bevy::ecs::message::Message;
use bevy::prelude::*;
use common::world::chunk::ChunkCoord;

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

/// The server's authoritative world clock.
#[derive(Message)]
pub struct InboundSyncTimeMessage {
    pub total_days: u64,
    pub time_of_day: f32,
    pub paused: bool,
}

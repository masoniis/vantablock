use bevy::ecs::message::Message;
use bevy::prelude::*;
use shared::world::chunk::ChunkCoord;

#[derive(Message)]
pub struct WelcomeMessage {
    pub spawn_pos: Vec3,
}

#[derive(Message)]
pub struct ReceivedCompressedChunkMessage {
    pub coord: ChunkCoord,
    pub data: Vec<u8>,
}

#[derive(Message)]
pub struct ReceivedDecompressedChunkMessage {
    pub coord: ChunkCoord,
    pub data: Vec<u8>,
}

use bevy::ecs::message::Message;
use bevy::prelude::*;
use shared::simulation::chunk::ChunkCoord;

#[derive(Message)]
pub struct WelcomeEvent {
    pub spawn_pos: Vec3,
}

#[derive(Message)]
pub struct ReceivedChunkDataEvent {
    pub coord: ChunkCoord,
    pub data: Vec<u8>,
}

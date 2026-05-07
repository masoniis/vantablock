use crate::{player::PlayerAction, world::chunk::ChunkCoord};
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// A protocol defining all the direct client messages that a client communicates
/// to the server. Note that this only defines the boundaries for message comms
/// while the client also has other means of communication like replication.
#[derive(Message, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ClientMessage {
    /// A request for the server to send block data for a specific chunk.
    RequestChunk(ChunkCoord),
    /// Updates the server on the player's current view orientation/camera state.
    /// Necessary for authoritative targeting or raycasting calculations.
    UpdateView { forward: Vec3 },
    /// Intent to break a block at the specified position.
    BreakBlock { position: IVec3 },
    /// Intent to place a block at the specified position.
    PlaceBlock { position: IVec3, block_id: u8 },
}

/// A protocol defining all the direct server messages that a server communicates
/// to its clients. Note that this only defines the boundaries for message comms
/// while the server also has other means of communication like replication.
#[derive(Message, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ServerMessage {
    /// Direct block update for a specific world position. Represents generalized state change broadcast.
    BlockUpdate { position: IVec3, block_id: u8 },
    /// Bulk data for a chunk, typically compressed or encoded from ChunkVolumeData.
    ChunkData {
        coord: ChunkCoord,
        data: Vec<u8>, // u8 matches BlockId
    },
    /// Synchronizes the authoritative game time across all clients.
    SyncTime { game_time: f32, tick: u64 },
}

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

pub struct NetMessagesPlugin;

impl Plugin for NetMessagesPlugin {
    fn build(&self, app: &mut App) {
        app.register_message::<ClientMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<ServerMessage>()
            .add_direction(NetworkDirection::ServerToClient);
    }
}

use crate::simulation::chunk::ChunkCoord;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    /// Initial state sent to the client upon joining.
    Welcome { entity_id: Entity, spawn_pos: Vec3 },
    /// Direct voxel update for a specific world position.
    VoxelUpdate { position: IVec3, block_id: u16 },
    /// Bulk data for a chunk, typically compressed or encoded from ChunkVolumeData.
    ChunkData {
        coord: ChunkCoord,
        data: Vec<u16>, // TODO: may want a more efficient encoding
    },
    /// Synchronizes the authoritative game time across all clients.
    SyncTime { game_time: f32, tick: u64 },
}

use bevy::ecs::prelude::Message;
use shared::prelude::IVec3;

/// An event that is sent when a voxel should be broken.
#[derive(Message, Clone)]
pub struct BreakVoxelEvent {
    /// The world position of the voxel to break.
    pub world_pos: IVec3,
}

/// An event that is sent when a voxel should be placed.
#[derive(Message, Clone)]
pub struct PlaceVoxelEvent {
    /// The world position to place a voxel.
    pub target_pos: IVec3,
    /// The ID of the block to place.
    pub block_id: u8,
}

use bevy::ecs::prelude::Message;
use shared::prelude::IVec3;

/// An event that is sent when a block should be broken.
#[derive(Message, Clone)]
pub struct BreakBlockEvent {
    /// The world position of the block to break.
    pub world_pos: IVec3,
}

/// An event that is sent when a block should be placed.
#[derive(Message, Clone)]
pub struct PlaceBlockEvent {
    /// The world position to place a block.
    pub target_pos: IVec3,
    /// The ID of the block to place.
    pub block_id: u8,
}

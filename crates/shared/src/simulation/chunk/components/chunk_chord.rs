use crate::prelude::*;
use crate::simulation::chunk::{CHUNK_DEPTH, CHUNK_HEIGHT, CHUNK_SIDE_LENGTH, CHUNK_WIDTH};
use bevy::ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use std::fmt;

const BIT_SHIFT: i32 = CHUNK_SIDE_LENGTH.trailing_zeros() as i32;

/// Stores the coordinate of the chunk an entity is currently in.
#[derive(Component, Clone, Deref, DerefMut, Debug)]
pub struct ChunkCoord {
    pub pos: IVec3,
}

impl fmt::Display for ChunkCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x {}y {}z", self.pos.x, self.pos.y, self.pos.z)
    }
}

impl ChunkCoord {
    pub fn get_block_world_pos(&self, local_pos: IVec3) -> IVec3 {
        IVec3::new(
            (self.pos.x << BIT_SHIFT) + local_pos.x,
            (self.pos.y << BIT_SHIFT) + local_pos.y,
            (self.pos.z << BIT_SHIFT) + local_pos.z,
        )
    }

    pub fn as_world_pos(&self) -> IVec3 {
        IVec3::new(
            self.pos.x << BIT_SHIFT,
            self.pos.y << BIT_SHIFT,
            self.pos.z << BIT_SHIFT,
        )
    }

    // INFO: -----------------------------
    //         static method utils
    // -----------------------------------

    /// Helper to convert world coordinates to chunk/local coordinates
    pub fn world_to_chunk_and_local_pos(world_pos: IVec3) -> (IVec3, IVec3) {
        let chunk_coord = IVec3::new(
            world_pos.x >> BIT_SHIFT,
            world_pos.y >> BIT_SHIFT,
            world_pos.z >> BIT_SHIFT,
        );

        let local_pos = IVec3::new(
            world_pos.x & (CHUNK_WIDTH as i32 - 1),
            world_pos.y & (CHUNK_HEIGHT as i32 - 1),
            world_pos.z & (CHUNK_DEPTH as i32 - 1),
        );

        (chunk_coord, local_pos)
    }

    /// Convert a world position to chunk coordinate
    pub fn world_to_chunk_pos(world_pos: Vec3) -> IVec3 {
        let int_pos = world_pos.floor().as_ivec3();

        IVec3::new(
            int_pos.x >> BIT_SHIFT,
            int_pos.y >> BIT_SHIFT,
            int_pos.z >> BIT_SHIFT,
        )
    }
}

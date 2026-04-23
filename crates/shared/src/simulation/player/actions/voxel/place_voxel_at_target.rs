use crate::prelude::*;
use crate::simulation::chunk::ChunkStateManager;
use crate::simulation::{
    block::block_registry::SOLID_BLOCK_ID,
    chunk::{
        CHUNK_SIDE_LENGTH,
        components::{ChunkBlocksComponent, ChunkCoord, ChunkMeshDirty},
    },
};
use bevy::ecs::prelude::Res;
use bevy::ecs::prelude::{Commands, Message, MessageReader, Query};

/// An event that is sent when a voxel should be placed.
#[derive(Message, Clone)]
pub struct PlaceVoxelEvent {
    /// The world position to place a voxel.
    pub target_pos: IVec3,
}

/// A system that handles the `PlaceVoxelEvent`.
pub fn handle_place_voxel_events_system(
    // input
    mut events: MessageReader<PlaceVoxelEvent>,
    chunk_manager: Res<ChunkStateManager>,

    // output
    mut chunks: Query<&mut ChunkBlocksComponent>,
    mut commands: Commands,
) {
    for event in events.read() {
        let new_block_pos = event.target_pos;
        let chunk_pos = ChunkCoord::world_to_chunk_pos(new_block_pos.as_vec3());

        if let Some(entity) = chunk_manager.get_entity(chunk_pos)
            && let Ok(mut chunk_blocks) = chunks.get_mut(entity)
        {
            let local_pos = new_block_pos - (chunk_pos * CHUNK_SIDE_LENGTH as i32);

            let mut writer = chunk_blocks.get_writer();
            writer.set_data(
                local_pos.x as usize,
                local_pos.y as usize,
                local_pos.z as usize,
                SOLID_BLOCK_ID,
            );

            // mark primary chunk as dirty
            commands.entity(entity).insert(ChunkMeshDirty);

            // mark any neighbors as dirty if relevant
            let max_idx = (CHUNK_SIDE_LENGTH - 1) as i32;
            let mut neighbor_coords_to_dirty = Vec::with_capacity(3);

            if local_pos.x == 0 {
                neighbor_coords_to_dirty.push(chunk_pos - IVec3::X);
            } else if local_pos.x == max_idx {
                neighbor_coords_to_dirty.push(chunk_pos + IVec3::X);
            }

            if local_pos.y == 0 {
                neighbor_coords_to_dirty.push(chunk_pos - IVec3::Y);
            } else if local_pos.y == max_idx {
                neighbor_coords_to_dirty.push(chunk_pos + IVec3::Y);
            }

            if local_pos.z == 0 {
                neighbor_coords_to_dirty.push(chunk_pos - IVec3::Z);
            } else if local_pos.z == max_idx {
                neighbor_coords_to_dirty.push(chunk_pos + IVec3::Z);
            }

            for neighbor_coord in neighbor_coords_to_dirty {
                if let Some(neighbor_entity) = chunk_manager.get_entity(neighbor_coord) {
                    commands.entity(neighbor_entity).insert(ChunkMeshDirty);
                }
            }
        }
    }
}

use crate::prelude::*;
use crate::simulation::block::TargetedBlock;
use crate::simulation::chunk::ChunkStateManager;
use crate::simulation::{
    block::block_registry::AIR_BLOCK_ID,
    chunk::{
        CHUNK_SIDE_LENGTH,
        components::{ChunkBlocksComponent, ChunkCoord, ChunkMeshDirty},
    },
};
use bevy::ecs::prelude::{Commands, Message, MessageReader, Query};
use bevy::ecs::prelude::{MessageWriter, Res};

/// An event that is sent when a voxel should be broken.
#[derive(Message, Clone)]
pub struct BreakVoxelEvent {
    /// The world position of the voxel to break.
    pub world_pos: IVec3,
}

/// Fires a `BreakVoxelEvent` for the currently targeted block.
pub fn break_targeted_voxel_system(
    targeted_block: Res<TargetedBlock>,
    mut break_voxel_writer: MessageWriter<BreakVoxelEvent>,
) {
    if let Some(voxel_pos) = targeted_block.position {
        break_voxel_writer.write(BreakVoxelEvent {
            world_pos: voxel_pos,
        });
    }
}

/// A system that handles the `BreakVoxelEvent`.
pub fn handle_break_voxel_events_system(
    // input
    mut events: MessageReader<BreakVoxelEvent>,
    chunk_manager: Res<ChunkStateManager>,

    // output
    mut chunks: Query<&mut ChunkBlocksComponent>,
    mut commands: Commands,
) {
    for event in events.read() {
        let chunk_pos = ChunkCoord::world_to_chunk_pos(event.world_pos.as_vec3());

        if let Some(entity) = chunk_manager.get_entity(chunk_pos)
            && let Ok(mut chunk_blocks) = chunks.get_mut(entity)
        {
            let local_pos = event.world_pos - (chunk_pos * CHUNK_SIDE_LENGTH as i32);

            let mut writer = chunk_blocks.get_writer();
            writer.set_data(
                local_pos.x as usize,
                local_pos.y as usize,
                local_pos.z as usize,
                AIR_BLOCK_ID,
            );

            // mark the primary chunk as dirty
            commands.entity(entity).insert(ChunkMeshDirty);

            // mark any neighbors as dirty if we are on the edge
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

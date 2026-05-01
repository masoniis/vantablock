use crate::player::{BreakVoxelEvent, LocalPlayer, PlaceVoxelEvent, TargetedBlock};
use bevy::ecs::prelude::{Commands, Entity, MessageReader, MessageWriter};
use bevy::ecs::prelude::{Query, Res, With};
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::network::protocol::{BlockUpdates, ClientMessage, ServerMessage};
use shared::prelude::IVec3;
use shared::world::block::block_registry::AIR_BLOCK_ID;
use shared::world::chunk::{
    CHUNK_SIDE_LENGTH,
    components::{ChunkBlocksComponent, ChunkCoord, ChunkMeshDirty},
};
use std::collections::HashMap;

/// Fires a `BreakVoxelEvent` for the currently targeted block and sends a
/// `ClientMessage::BreakBlock` to the server.
pub fn break_targeted_voxel_system(
    targeted_block: Res<TargetedBlock>,
    mut break_voxel_writer: MessageWriter<BreakVoxelEvent>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<LocalPlayer>>,
) {
    if let Some(voxel_pos) = targeted_block.position {
        // send local event for instant Client-Side Prediction (CSP)
        break_voxel_writer.write(BreakVoxelEvent {
            world_pos: voxel_pos,
        });

        // send networked intent to the server
        if let Ok(mut sender) = sender_query.single_mut() {
            sender.send::<BlockUpdates>(ClientMessage::BreakBlock {
                position: voxel_pos,
            });
        }
    }
}

/// Listens for `ServerMessage::VoxelUpdate` from the server (representing other players' actions or server-side changes)
/// and translates them into local voxel events.
pub fn handle_incoming_voxel_updates(
    mut query: Query<&mut MessageReceiver<ServerMessage>, With<LocalPlayer>>,
    mut break_voxel_writer: MessageWriter<BreakVoxelEvent>,
    mut place_voxel_writer: MessageWriter<PlaceVoxelEvent>,
) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
            if let ServerMessage::VoxelUpdate { position, block_id } = message {
                if block_id == AIR_BLOCK_ID {
                    break_voxel_writer.write(BreakVoxelEvent {
                        world_pos: position,
                    });
                } else {
                    place_voxel_writer.write(PlaceVoxelEvent {
                        target_pos: position,
                        block_id,
                    });
                }
            }
        }
    }
}

/// A system that handles the `BreakVoxelEvent` by mutating local chunk data
/// and marking chunks as dirty for remeshing.
pub fn handle_break_voxel_events_system(
    // input
    mut events: MessageReader<BreakVoxelEvent>,
    chunk_query: Query<(&ChunkCoord, Entity)>,

    // output
    mut blocks_query: Query<&mut ChunkBlocksComponent>,
    mut commands: Commands,
) {
    if events.is_empty() {
        return;
    }

    // build a temporary map for lookups
    let mut entity_map = HashMap::new();
    for (coord, entity) in chunk_query.iter() {
        entity_map.insert(coord.pos, entity);
    }

    for event in events.read() {
        let chunk_pos = ChunkCoord::world_to_chunk_pos(event.world_pos.as_vec3());

        if let Some(&entity) = entity_map.get(&chunk_pos)
            && let Ok(mut chunk_blocks) = blocks_query.get_mut(entity)
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
                if let Some(&neighbor_entity) = entity_map.get(&neighbor_coord) {
                    commands.entity(neighbor_entity).insert(ChunkMeshDirty);
                }
            }
        }
    }
}

/// Fires a `PlaceVoxelEvent` for the currently targeted block and sends a
/// `ClientMessage::PlaceBlock` to the server.
pub fn place_targeted_voxel_system(
    targeted_block: Res<TargetedBlock>,
    mut place_voxel_writer: MessageWriter<PlaceVoxelEvent>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<LocalPlayer>>,
) {
    if let (Some(voxel_pos), Some(normal)) = (targeted_block.position, targeted_block.normal) {
        let target_pos = voxel_pos + normal;
        let block_id = 1; // TODO: Use actual selected block ID from an inventory resource

        // send local event for CSP
        place_voxel_writer.write(PlaceVoxelEvent {
            target_pos,
            block_id,
        });

        // send networked intent to the server
        if let Ok(mut sender) = sender_query.single_mut() {
            sender.send::<BlockUpdates>(ClientMessage::PlaceBlock {
                position: target_pos,
                block_id,
            });
        }
    }
}

/// A system that handles the `PlaceVoxelEvent` by mutating local chunk data
/// and marking chunks as dirty for remeshing.
pub fn handle_place_voxel_events_system(
    // input
    mut events: MessageReader<PlaceVoxelEvent>,
    chunk_query: Query<(&ChunkCoord, Entity)>,

    // output
    mut blocks_query: Query<&mut ChunkBlocksComponent>,
    mut commands: Commands,
) {
    if events.is_empty() {
        return;
    }

    // build a temporary map for lookups
    let mut entity_map = HashMap::new();
    for (coord, entity) in chunk_query.iter() {
        entity_map.insert(coord.pos, entity);
    }

    for event in events.read() {
        let chunk_pos = ChunkCoord::world_to_chunk_pos(event.target_pos.as_vec3());

        if let Some(&entity) = entity_map.get(&chunk_pos)
            && let Ok(mut chunk_blocks) = blocks_query.get_mut(entity)
        {
            let local_pos = event.target_pos - (chunk_pos * CHUNK_SIDE_LENGTH as i32);

            let mut writer = chunk_blocks.get_writer();
            writer.set_data(
                local_pos.x as usize,
                local_pos.y as usize,
                local_pos.z as usize,
                event.block_id,
            );

            // mark primary chunk as dirty
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
                if let Some(&neighbor_entity) = entity_map.get(&neighbor_coord) {
                    commands.entity(neighbor_entity).insert(ChunkMeshDirty);
                }
            }
        }
    }
}

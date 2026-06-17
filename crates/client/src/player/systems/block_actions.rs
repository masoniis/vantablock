use crate::player::{BreakBlockEvent, LocalPlayer, PlaceBlockEvent, TargetedBlock};
use bevy::ecs::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::{
    network::protocol::{BlockUpdates, ClientMessage, ServerMessage},
    prelude::IVec3,
    world::block::block_registry::AIR_BLOCK_ID,
    world::chunk::{
        CHUNK_SIDE_LENGTH,
        components::{ChunkBlocksComponent, ChunkCoord, ChunkMeshDirty},
    },
};
use std::collections::HashMap;

/// Fires a `BreakBlockEvent` for the currently targeted block and sends a
/// `ClientMessage::BreakBlock` to the server.
pub fn break_targeted_block_system(
    targeted_block: Res<TargetedBlock>,
    mut break_block_writer: MessageWriter<BreakBlockEvent>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<LocalPlayer>>,
) {
    if let Some(block_pos) = targeted_block.position {
        // send local event for instant Client-Side Prediction (CSP)
        break_block_writer.write(BreakBlockEvent {
            world_pos: block_pos,
        });

        // send networked intent to the server
        if let Ok(mut sender) = sender_query.single_mut() {
            sender.send::<BlockUpdates>(ClientMessage::BreakBlock {
                position: block_pos,
            });
        }
    }
}

/// Listens for `ServerMessage::BlockUpdate` from the server (representing other players' actions or server-side changes)
/// and translates them into local block events.
pub fn handle_incoming_block_updates(
    mut query: Query<&mut MessageReceiver<ServerMessage>, With<LocalPlayer>>,
    mut break_block_writer: MessageWriter<BreakBlockEvent>,
    mut place_block_writer: MessageWriter<PlaceBlockEvent>,
) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
            if let ServerMessage::BlockUpdate { position, block_id } = message {
                if block_id == AIR_BLOCK_ID {
                    break_block_writer.write(BreakBlockEvent {
                        world_pos: position,
                    });
                } else {
                    place_block_writer.write(PlaceBlockEvent {
                        target_pos: position,
                        block_id,
                    });
                }
            }
        }
    }
}

/// A system that handles the `BreakBlockEvent` by mutating local chunk data
/// and marking chunks as dirty for remeshing.
pub fn handle_break_block_events_system(
    // input
    mut events: MessageReader<BreakBlockEvent>,
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

/// Fires a `PlaceBlockEvent` for the currently targeted block and sends a
/// `ClientMessage::PlaceBlock` to the server.
pub fn place_targeted_block_system(
    targeted_block: Res<TargetedBlock>,
    mut place_block_writer: MessageWriter<PlaceBlockEvent>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<LocalPlayer>>,
) {
    if let (Some(block_pos), Some(normal)) = (targeted_block.position, targeted_block.normal) {
        let target_pos = block_pos + normal;
        let block_id = 1;

        // send local event for CSP
        place_block_writer.write(PlaceBlockEvent {
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

/// A system that handles the `PlaceBlockEvent` by mutating local chunk data
/// and marking chunks as dirty for remeshing.
pub fn handle_place_block_events_system(
    // input
    mut events: MessageReader<PlaceBlockEvent>,
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

use crate::{
    prelude::*,
    render::chunk::manager::{ClientChunkManager, ClientChunkState},
};
use bevy::{
    ecs::prelude::*,
    math::IVec3,
    prelude::{Camera, Camera3d},
};
use lightyear::prelude::{Client, MessageSender};
use shared::{
    network::{ChunkData, ClientMessage},
    world::chunk::{ChunkCoord, LOAD_DISTANCE, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK},
};
use std::collections::HashSet;

/// A system that determines which chunks should be loaded based on camera position.
/// This system manages chunk lifecycle transitions from Unloaded to AwaitingData, requesting
/// each newly awaited chunk from the server.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_loading_targets_system(
    // input
    camera_query: Query<(&Camera, &ChunkCoord), With<Camera3d>>,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<Client>>,
    mut commands: Commands,
) {
    // without a connection there is nobody to request chunks from
    let Ok(mut sender) = sender_query.single_mut() else {
        return;
    };

    let mut active_camera_chunk_pos = None;
    for (camera, chunk_coord) in camera_query.iter() {
        if camera.is_active {
            active_camera_chunk_pos = Some(chunk_coord.pos);
            break;
        }
    }

    let Some(camera_chunk_pos) = active_camera_chunk_pos else {
        return;
    };

    let mut desired_load_chunks = HashSet::new();

    for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
        for z in -LOAD_DISTANCE..=LOAD_DISTANCE {
            for x in -LOAD_DISTANCE..=LOAD_DISTANCE {
                let coord = IVec3::new(camera_chunk_pos.x + x, y, camera_chunk_pos.z + z);
                desired_load_chunks.insert(coord);
            }
        }
    }

    // handle unloading
    let mut coords_to_remove = Vec::new();
    for (coord, state) in chunk_manager.chunk_states.iter() {
        if !desired_load_chunks.contains(coord) {
            // chunk is outside load distance, unload it completely
            match state {
                ClientChunkState::AwaitingData => {
                    // nothing to despawn
                }
                ClientChunkState::DataReady { entity }
                | ClientChunkState::NeedsMeshing { entity }
                | ClientChunkState::Meshing { entity }
                | ClientChunkState::MeshComplete { entity } => {
                    debug!(target:"chunk_loading", "Unloading chunk at {:?} (Entity: {:?})", coord, entity);
                    commands.entity(*entity).despawn();
                }
            }
            coords_to_remove.push(*coord);
        }
    }

    for coord in coords_to_remove {
        chunk_manager.mark_as_unloaded(coord);
    }

    // identify new chunks to load
    let mut requested = Vec::new();
    for coord in desired_load_chunks {
        if !chunk_manager.is_chunk_present_or_loading(coord) {
            chunk_manager.mark_as_awaiting_data(coord);
            requested.push(ChunkCoord { pos: coord });
        }
    }

    if !requested.is_empty() {
        debug!(target:"chunk_loading", "Requesting {} chunks from the server", requested.len());
        sender.send::<ChunkData>(ClientMessage::RequestChunks(requested));
    }
}

/// Despawns every client chunk and forgets its state when leaving the game, so a later
/// session doesn't render or skip requesting chunks from the previous world.
pub fn unload_all_chunks_system(
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    for state in chunk_manager.chunk_states.values() {
        match state {
            ClientChunkState::AwaitingData => {}
            ClientChunkState::DataReady { entity }
            | ClientChunkState::NeedsMeshing { entity }
            | ClientChunkState::Meshing { entity }
            | ClientChunkState::MeshComplete { entity } => {
                commands.entity(*entity).despawn();
            }
        }
    }
    chunk_manager.chunk_states.clear();
}

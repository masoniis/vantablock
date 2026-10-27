use crate::{
    prelude::*,
    world::chunk::{
        ClientChunkManager,
        range::{
            ChunkCameraQuery, active_camera_chunk, chunks_in_horizontal_range,
            is_within_horizontal_range,
        },
    },
};
use bevy::ecs::prelude::*;
use common::{
    network::{ChunkData, ClientMessage},
    world::chunk::{ChunkCoord, LOAD_DISTANCE},
};
use lightyear::prelude::{Client, MessageSender};

/// A system that determines which chunks should be loaded based on camera position.
/// Unloads chunks that left the load distance and requests each newly awaited chunk from the
/// server.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_loading_targets_system(
    // input
    camera_query: ChunkCameraQuery,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<Client>>,
    mut commands: Commands,
) {
    // without a connection there is nobody to request chunks from
    let Ok(mut sender) = sender_query.single_mut() else {
        return;
    };

    let Some(camera_chunk_pos) = active_camera_chunk(&camera_query) else {
        return;
    };

    // handle unloading
    chunk_manager.chunk_states.retain(|coord, state| {
        if is_within_horizontal_range(camera_chunk_pos, *coord, LOAD_DISTANCE) {
            return true;
        }
        if let Some(entity) = state.entity() {
            debug!(target:"chunk_loading", "Unloading chunk at {:?} (Entity: {:?})", coord, entity);
            commands.entity(entity).despawn();
        }
        false
    });

    // identify new chunks to load
    let mut requested = Vec::new();
    for coord in chunks_in_horizontal_range(camera_chunk_pos, LOAD_DISTANCE) {
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
    for entity in chunk_manager
        .chunk_states
        .drain()
        .filter_map(|(_, s)| s.entity())
    {
        commands.entity(entity).despawn();
    }
}

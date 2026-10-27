use crate::{
    prelude::*,
    world::chunk::{
        ClientChunkMap,
        range::{
            StreamingAnchorQuery, chunks_in_horizontal_range, is_within_horizontal_range,
            streaming_center,
        },
    },
};
use bevy::ecs::prelude::*;
use common::{
    network::{ChunkData, ClientMessage},
    world::chunk::{ChunkCoord, LOAD_DISTANCE},
};
use lightyear::prelude::{Client, MessageSender};

/// A system that determines which chunks should be loaded around the streaming anchor.
/// Unloads chunks that left the load distance and requests each newly awaited chunk from the
/// server.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_loading_targets_system(
    // input
    anchor: StreamingAnchorQuery,

    // output
    mut chunk_map: ResMut<ClientChunkMap>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<Client>>,
    mut commands: Commands,
) {
    // without a connection there is nobody to request chunks from
    let Ok(mut sender) = sender_query.single_mut() else {
        return;
    };

    let Some(center) = streaming_center(&anchor) else {
        return;
    };

    // handle unloading
    chunk_map.chunk_states.retain(|coord, state| {
        if is_within_horizontal_range(center, *coord, LOAD_DISTANCE) {
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
    for coord in chunks_in_horizontal_range(center, LOAD_DISTANCE) {
        if !chunk_map.is_chunk_present_or_loading(coord) {
            chunk_map.mark_as_awaiting_data(coord);
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
pub fn unload_all_chunks_system(mut chunk_map: ResMut<ClientChunkMap>, mut commands: Commands) {
    for entity in chunk_map
        .chunk_states
        .drain()
        .filter_map(|(_, s)| s.entity())
    {
        commands.entity(entity).despawn();
    }
}

use crate::{prelude::*, world::chunk::ClientChunkMap};
use bevy::ecs::prelude::*;
use common::world::chunk::{ChunkCoord, ChunkMeshDirty};

/// Queues chunks changed by block edits for a remesh.
pub fn handle_dirty_chunks_system(
    // input
    dirty_chunks_query: Query<(Entity, &ChunkCoord), With<ChunkMeshDirty>>,

    // output
    mut commands: Commands,
    mut chunk_map: ResMut<ClientChunkMap>,
) {
    for (entity, coord) in dirty_chunks_query.iter() {
        commands.entity(entity).remove::<ChunkMeshDirty>();

        // a chunk the chunk map no longer tracks with this entity is being unloaded
        if chunk_map.get_entity(coord.pos) == Some(entity) {
            trace!("Chunk {} was edited, queueing a remesh", coord.pos);
            chunk_map.mark_as_needs_meshing(coord.pos, entity);
        }
    }
}

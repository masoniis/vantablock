use crate::prelude::*;
use crate::render::chunk::manager::ClientChunkManager;
use crate::render::chunk::tasks::{CheckForMeshing, WantsMeshing};
use bevy::ecs::prelude::*;
use shared::world::chunk::{ChunkCoord, ChunkMeshDirty};

/// A system that detects chunks marked as dirty and prepares them for re-meshing.
pub fn handle_dirty_chunks_system(
    // input
    dirty_chunks_query: Query<(Entity, &ChunkCoord), With<ChunkMeshDirty>>,

    // output
    mut commands: Commands,
    mut chunk_manager: ResMut<ClientChunkManager>,
) {
    for (entity, coord) in dirty_chunks_query.iter() {
        trace!(
            "Chunk {:?} at {} was marked as dirty, preparing for re-meshing.",
            entity, coord.pos
        );

        chunk_manager.mark_as_needs_meshing(coord.pos, entity);

        commands
            .entity(entity)
            .insert((WantsMeshing, CheckForMeshing))
            .remove::<ChunkMeshDirty>();
    }
}

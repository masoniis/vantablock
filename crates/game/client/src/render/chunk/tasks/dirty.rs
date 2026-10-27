use crate::{prelude::*, render::chunk::ChunkMesh};
use bevy::ecs::prelude::*;
use common::world::chunk::{ChunkCoord, ChunkMeshDirty};

/// Queues chunks changed by block edits for a remesh.
///
/// Only chunks that already have a mesh state are re-queued; any other chunk is meshed with its
/// current data once the scheduler queues it.
pub fn handle_dirty_chunks_system(
    // input
    mut dirty_chunks_query: Query<
        (Entity, &ChunkCoord, Option<&mut ChunkMesh>),
        With<ChunkMeshDirty>,
    >,

    // output
    mut commands: Commands,
) {
    for (entity, coord, mesh) in &mut dirty_chunks_query {
        commands.entity(entity).remove::<ChunkMeshDirty>();
        if let Some(mut mesh) = mesh {
            trace!("Chunk {} was edited, queueing a remesh", coord.pos);
            *mesh = ChunkMesh::Queued;
        }
    }
}

use crate::{prelude::*, render::chunk::ChunkMesh, world::chunk::ChunkEdited};
use bevy::ecs::prelude::*;
use common::world::chunk::ChunkCoord;

/// Queues edited chunks for a remesh.
///
/// Only chunks that already have a mesh state are re-queued; any other chunk is meshed with its
/// current data once the scheduler queues it.
pub fn handle_dirty_chunks_system(
    // input
    mut dirty_chunks_query: Query<(Entity, &ChunkCoord, Option<&mut ChunkMesh>), With<ChunkEdited>>,

    // output
    mut commands: Commands,
) {
    for (entity, coord, mesh) in &mut dirty_chunks_query {
        commands.entity(entity).remove::<ChunkEdited>();
        if let Some(mut mesh) = mesh {
            trace!("Chunk {} was edited, queueing a remesh", coord.pos);
            *mesh = ChunkMesh::Queued;
        }
    }
}

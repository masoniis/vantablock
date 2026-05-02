use crate::{prelude::*, world::chunk::ChunkCoord};
use bevy::{ecs::system::Query, transform::components::GlobalTransform};

/// A system to that updates the active camera's chunk chord based on its position.
#[instrument(skip_all)]
pub fn update_chunk_coords_system(mut query: Query<(&GlobalTransform, &mut ChunkCoord)>) {
    for (transform, mut vicinity) in query.iter_mut() {
        // update chunk chord if it is different
        let new_chunk_pos = ChunkCoord::world_to_chunk_pos(transform.translation());
        if new_chunk_pos != vicinity.pos {
            debug!(
                target: "camera_chunk",
                "Entity crossed chunk boundary. Old: {:?}, New: {:?}",
                vicinity.pos, new_chunk_pos
            );
            vicinity.pos = new_chunk_pos;
        }
    }
}

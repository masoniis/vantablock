use crate::{player::components::LogicalPosition, prelude::*, world::chunk::ChunkCoord};
use bevy::{ecs::system::Query, transform::components::GlobalTransform};

/// A system that updates the `ChunkCoord` of entities based on their position.
/// It prefers `LogicalPosition` (authoritative simulation position) but falls back
/// to `GlobalTransform` for entities without logical simulation state (like cameras).
#[instrument(skip_all)]
pub fn update_chunk_coords_system(
    mut query: Query<(&GlobalTransform, &mut ChunkCoord, Option<&LogicalPosition>)>,
) {
    for (transform, mut vicinity, logical_pos) in query.iter_mut() {
        let pos = logical_pos
            .map(|lp| lp.0)
            .unwrap_or_else(|| transform.translation());

        let new_chunk_pos = ChunkCoord::world_to_chunk_pos(pos);
        if new_chunk_pos != vicinity.pos {
            debug!(
                target: "chunk_coords",
                "Entity crossed chunk boundary. Old: {:?}, New: {:?}",
                vicinity.pos, new_chunk_pos
            );
            vicinity.pos = new_chunk_pos;
        }
    }
}

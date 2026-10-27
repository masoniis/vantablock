use bevy::ecs::prelude::*;
use bevy::math::IVec3;
use std::collections::HashMap;

/// Resource tracking the entities associated with active chunk coordinates.
///
/// This map only handles spatial lookup (coord -> Entity).
/// The actual state of the chunk is handled via marker components (NeedsGenerating, Generating, ActiveChunk, EmptyChunk).
#[derive(Resource, Default, Debug)]
pub struct ChunkMap {
    /// Map tracking the entities of all non-unloaded chunks.
    pub chunks: HashMap<IVec3, Entity>,
}

impl ChunkMap {
    /// Gets the Entity for a chunk, if that chunk is tracked.
    pub fn get_chunk(&self, coord: IVec3) -> Option<Entity> {
        self.chunks.get(&coord).copied()
    }

    /// Checks if a chunk is registered in the map.
    pub fn is_chunk_present(&self, coord: IVec3) -> bool {
        self.chunks.contains_key(&coord)
    }

    /// Registers a new chunk entity at the given coordinate.
    pub fn register_chunk(&mut self, coord: IVec3, entity: Entity) {
        self.chunks.insert(coord, entity);
    }

    /// Unregisters a chunk, removing it from spatial tracking.
    pub fn unregister_chunk(&mut self, coord: IVec3) {
        self.chunks.remove(&coord);
    }
}

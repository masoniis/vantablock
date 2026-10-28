use bevy::ecs::prelude::*;
use bevy::math::IVec3;
use std::collections::HashMap;

/// Where a tracked chunk is in the client's data lifecycle.
///
/// Only data: whether a chunk is meshed is the `ChunkMesh` component that `render::chunk`
/// puts on the chunk entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientChunkState {
    /// Requested from the server; no data yet.
    AwaitingData,
    /// Its block data arrived and lives on this entity.
    Loaded { entity: Entity },
}

impl ClientChunkState {
    /// The chunk's entity, once its data has arrived.
    pub fn entity(&self) -> Option<Entity> {
        match *self {
            ClientChunkState::AwaitingData => None,
            ClientChunkState::Loaded { entity } => Some(entity),
        }
    }
}

/// Every chunk the client tracks, by coordinate, and whether its data has arrived.
///
/// This is the single source of truth for which chunks the client holds: a chunk is tracked
/// exactly when it has an entry here, and its entity is the one recorded here. Everything
/// else about a chunk (its blocks, its mesh state, its meshes) is a component on that entity,
/// so despawning the entity on unload removes all of it at once. It mirrors the server's
/// `ChunkMap`.
#[derive(Resource, Default, Debug)]
pub struct ClientChunkMap {
    /// The state of every tracked chunk. Untracked chunks are absent.
    pub chunk_states: HashMap<IVec3, ClientChunkState>,
}

impl ClientChunkMap {
    /// The chunk's state, if it is tracked.
    pub fn get_state(&self, coord: IVec3) -> Option<ClientChunkState> {
        self.chunk_states.get(&coord).copied()
    }

    /// The chunk's entity, if it is tracked and its data has arrived.
    pub fn get_entity(&self, coord: IVec3) -> Option<Entity> {
        self.chunk_states
            .get(&coord)
            .and_then(|state| state.entity())
    }

    /// Whether the chunk is tracked at all (awaiting data or loaded).
    pub fn is_tracked(&self, coord: IVec3) -> bool {
        self.chunk_states.contains_key(&coord)
    }

    /// Records that the chunk was requested from the server.
    pub fn mark_as_awaiting_data(&mut self, coord: IVec3) {
        self.chunk_states
            .insert(coord, ClientChunkState::AwaitingData);
    }

    /// Records that the chunk's data arrived and lives on `entity`.
    pub fn mark_as_loaded(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states
            .insert(coord, ClientChunkState::Loaded { entity });
    }

    /// Stops tracking the chunk.
    pub fn mark_as_unloaded(&mut self, coord: IVec3) {
        self.chunk_states.remove(&coord);
    }
}

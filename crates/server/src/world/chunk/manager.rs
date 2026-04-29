use bevy::ecs::prelude::*;
use bevy::math::IVec3;
use std::collections::HashMap;

/// Represents the various states a chunk can be in during the server-side data lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerChunkState {
    /// Entity that can be acquired for generation
    NeedsGenerating { entity: Entity },
    /// Entity holds the generation Task component
    Generating { entity: Entity },
    /// Entity holds the generated data and is fully active.
    /// If entity is None, the chunk is empty but still considered "generated".
    Active { entity: Option<Entity> },
}

impl ServerChunkState {
    /// Returns the Entity associated with this chunk state, if it exists.
    pub fn entity(&self) -> Option<Entity> {
        match *self {
            ServerChunkState::NeedsGenerating { entity } => Some(entity),
            ServerChunkState::Generating { entity } => Some(entity),
            ServerChunkState::Active { entity } => entity,
        }
    }

    /// Returns true if the chunk has been generated (is Active).
    pub fn is_generated(&self) -> bool {
        matches!(*self, ServerChunkState::Active { .. })
    }
}

/// Resource tracking the data lifecycle of all active chunks on the server.
#[derive(Resource, Default, Debug)]
pub struct ServerChunkManager {
    /// Map tracking the state of all non-unloaded chunks.
    pub chunk_states: HashMap<IVec3, ServerChunkState>,
}

impl ServerChunkManager {
    /// Gets the current state of a chunk, if tracked.
    pub fn get_state(&self, coord: IVec3) -> Option<ServerChunkState> {
        self.chunk_states.get(&coord).copied()
    }

    /// Gets the Entity for a chunk, if that chunk is tracked and has an entity.
    pub fn get_entity(&self, coord: IVec3) -> Option<Entity> {
        self.chunk_states
            .get(&coord)
            .and_then(|state| state.entity())
    }

    /// Checks if a chunk exists in any loading or active state.
    pub fn is_chunk_present_or_loading(&self, coord: IVec3) -> bool {
        self.chunk_states.contains_key(&coord)
    }

    /// Marks that a chunk is requested to be loaded and needs generation.
    pub fn mark_as_needs_generating(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states
            .insert(coord, ServerChunkState::NeedsGenerating { entity });
    }

    /// Marks that a chunk is currently undergoing generation.
    pub fn mark_as_generating(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states
            .insert(coord, ServerChunkState::Generating { entity });
    }

    /// Marks that a chunk is fully generated and active.
    pub fn mark_as_active(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states.insert(
            coord,
            ServerChunkState::Active {
                entity: Some(entity),
            },
        );
    }

    /// Marks that a chunk is fully generated but is empty.
    pub fn mark_as_active_empty(&mut self, coord: IVec3) {
        self.chunk_states
            .insert(coord, ServerChunkState::Active { entity: None });
    }

    /// Called when a chunk is unloaded, removing it from tracking.
    pub fn mark_as_unloaded(&mut self, coord: IVec3) {
        self.chunk_states.remove(&coord);
    }
}

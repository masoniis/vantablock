use bevy::ecs::prelude::*;
use bevy::math::IVec3;
use std::collections::HashMap;

/// Represents the various states a chunk can be in during the client-side visual lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientChunkState {
    /// Chunk data is requested from server
    AwaitingData,
    /// Chunk data is received but not yet queued for meshing
    DataReady { entity: Entity },
    /// Chunk data is received, awaiting a mesh slot
    NeedsMeshing { entity: Entity },
    /// Chunk is currently undergoing meshing
    Meshing { entity: Entity },
    /// Chunk has completed its meshing pass (may or may not have produced a visible mesh).
    /// The entity contains the authoritative block data.
    MeshComplete { entity: Entity },
}

impl ClientChunkState {
    /// Returns the Entity associated with this chunk state, if it exists.
    pub fn entity(&self) -> Option<Entity> {
        match *self {
            ClientChunkState::AwaitingData => None,
            ClientChunkState::DataReady { entity } => Some(entity),
            ClientChunkState::NeedsMeshing { entity } => Some(entity),
            ClientChunkState::Meshing { entity } => Some(entity),
            ClientChunkState::MeshComplete { entity } => Some(entity),
        }
    }

    /// Returns true if the chunk has data (is at least DataReady).
    pub fn is_generated(&self) -> bool {
        !matches!(self, ClientChunkState::AwaitingData)
    }
}

/// Resource tracking the visual lifecycle (meshing/rendering) of all active chunks on the client.
#[derive(Resource, Default, Debug)]
pub struct ClientChunkManager {
    /// Map tracking the state of all non-unloaded chunks.
    pub chunk_states: HashMap<IVec3, ClientChunkState>,
}

impl ClientChunkManager {
    /// Gets the current state of a chunk, if tracked.
    pub fn get_state(&self, coord: IVec3) -> Option<ClientChunkState> {
        self.chunk_states.get(&coord).copied()
    }

    /// Gets the Entity for a chunk, if that chunk is tracked and has an entity.
    pub fn get_entity(&self, coord: IVec3) -> Option<Entity> {
        self.chunk_states
            .get(&coord)
            .and_then(|state| state.entity())
    }

    /// Checks if a chunk exists in any loading or rendered state.
    pub fn is_chunk_present_or_loading(&self, coord: IVec3) -> bool {
        self.chunk_states.contains_key(&coord)
    }

    /// Marks that a chunk is requested from the server but data hasn't arrived.
    pub fn mark_as_awaiting_data(&mut self, coord: IVec3) {
        self.chunk_states
            .insert(coord, ClientChunkState::AwaitingData);
    }

    /// Marks that a chunk has received data but is not yet queued for meshing.
    pub fn mark_as_data_ready(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states
            .insert(coord, ClientChunkState::DataReady { entity });
    }

    /// Marks that a chunk has received data and is queued to be meshed.
    pub fn mark_as_needs_meshing(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states
            .insert(coord, ClientChunkState::NeedsMeshing { entity });
    }

    /// Marks that a chunk has started the meshing process.
    pub fn mark_as_meshing(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states
            .insert(coord, ClientChunkState::Meshing { entity });
    }

    /// Marks that a chunk has completed its meshing pass.
    pub fn mark_as_mesh_complete(&mut self, coord: IVec3, entity: Entity) {
        self.chunk_states
            .insert(coord, ClientChunkState::MeshComplete { entity });
    }

    /// Called when a chunk is unloaded, removing it from tracking.
    pub fn mark_as_unloaded(&mut self, coord: IVec3) {
        self.chunk_states.remove(&coord);
    }

    /// An iterator over all chunks needing meshing.
    ///
    /// Useful for throttling the number of meshing tasks started per frame.
    pub fn iter_needs_meshing(&self) -> impl Iterator<Item = &IVec3> {
        self.chunk_states.iter().filter_map(|(coord, state)| {
            if matches!(state, ClientChunkState::NeedsMeshing { .. }) {
                Some(coord)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over all *existing* neighbors of a chunk.
    pub fn iter_neighbors(&self, coord: IVec3) -> impl Iterator<Item = NeighborInfo> + '_ {
        shared::world::chunk::NEIGHBOR_OFFSETS
            .into_iter()
            .filter_map(move |offset| {
                let neighbor_coord = coord + offset;
                self.get_state(neighbor_coord)
                    .and_then(|state| state.entity().map(|entity| (state, entity)))
                    .map(|(state, entity)| NeighborInfo {
                        offset,
                        coord: neighbor_coord,
                        state,
                        entity,
                    })
            })
    }
}

/// Holds information about a chunk's existing neighbor on the client.
pub struct NeighborInfo {
    pub offset: IVec3,
    pub coord: IVec3,
    pub state: ClientChunkState,
    pub entity: Entity,
}

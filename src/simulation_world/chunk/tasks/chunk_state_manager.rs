use bevy::math::IVec3;
use bevy_ecs::prelude::*;
use std::collections::HashMap;

use crate::simulation_world::chunk::{WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK};

/// Represents the various states a chunk can be in during loading and generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    /// Entity that can be acquired for generation
    NeedsGenerating { entity: Entity },
    /// Entity holds the generation Task component
    Generating { entity: Entity },
    /// Entity holds the generated data but is not queued for meshing
    ///
    /// This exists so that chunks can remain stagnant and hold data without
    /// being queued for meshing. Important for the "generation buffer" ring
    /// that extends past the mesh render distance (`LOAD_DISTANCE` const).
    DataReady { entity: Entity },
    /// Entity is awaiting a mesh slot
    WantsMeshing { entity: Entity },
    /// Entity holds the meshing Task component
    Meshing { entity: Entity },
    /// Entity is the final, rendered chunk
    Loaded { entity: Option<Entity> },
}

impl ChunkState {
    /// Returns the Entity associated with this chunk state.
    pub fn entity(&self) -> Option<Entity> {
        match *self {
            ChunkState::NeedsGenerating { entity } => Some(entity),
            ChunkState::Generating { entity } => Some(entity),
            ChunkState::DataReady { entity } => Some(entity),
            ChunkState::WantsMeshing { entity } => Some(entity),
            ChunkState::Meshing { entity } => Some(entity),
            ChunkState::Loaded { entity } => entity,
        }
    }
}

/// Offsets to find the 26 direct neighbors of a chunk.
///
/// If you imagine a chunk as the center of a 3x3 grid, then
/// this returns every offset in the grid except the 0-offset
/// which is the center chunk.
pub const NEIGHBOR_OFFSETS: [IVec3; 26] = {
    let mut offsets = [IVec3::ZERO; 26];

    let mut index = 0;
    let mut x = -1;
    while x <= 1 {
        let mut y = -1;
        while y <= 1 {
            let mut z = -1;
            while z <= 1 {
                if x != 0 || y != 0 || z != 0 {
                    offsets[index] = IVec3::new(x, y, z);
                    index += 1;
                }
                z += 1;
            }
            y += 1;
        }
        x += 1;
    }

    offsets
};

/// Holds information about a chunk's existing neighbor.
pub struct NeighborInfo {
    pub offset: IVec3,
    pub coord: IVec3,
    pub state: ChunkState,
    pub entity: Entity,
}

#[derive(Resource, Default, Debug)]
pub struct ChunkStateManager {
    /// Map tracking the state of all non-unloaded chunks.
    pub chunk_states: HashMap<IVec3, ChunkState>,
}

impl ChunkStateManager {
    /// Gets the current state of a chunk, if tracked.
    pub fn get_state(&self, coord: IVec3) -> Option<ChunkState> {
        self.chunk_states.get(&coord).copied()
    }

    /// Gets the Entity for a chunk, if that chunk is tracked.
    pub fn get_entity(&self, coord: IVec3) -> Option<Entity> {
        self.chunk_states.get(&coord).map(|state| state.entity())?
    }

    /// Checks if a chunk exists in any loading or loaded state.
    pub fn is_chunk_present_or_loading(&self, coord: IVec3) -> bool {
        self.chunk_states.contains_key(&coord)
    }

    /// Marks that a chunk is requested to be loaded.
    pub fn mark_as_needs_generating(&mut self, coord: IVec3, needs_generation_task_entity: Entity) {
        self.chunk_states.insert(
            coord,
            ChunkState::NeedsGenerating {
                entity: needs_generation_task_entity,
            },
        );
    }

    /// Marks that a chunk is currently undergoing generation.
    pub fn mark_as_generating(&mut self, coord: IVec3, generation_task_entity: Entity) {
        self.chunk_states.insert(
            coord,
            ChunkState::Generating {
                entity: generation_task_entity,
            },
        );
    }

    /// Called once a chunk's data is generated but not queued for meshing.
    pub fn mark_as_data_ready(&mut self, coord: IVec3, data_ready_entity: Entity) {
        self.chunk_states.insert(
            coord,
            ChunkState::DataReady {
                entity: data_ready_entity,
            },
        );
    }

    /// Called once a chunk's data is generated and is queued to be meshed.
    pub fn mark_as_needs_meshing(&mut self, coord: IVec3, needs_meshing_entity: Entity) {
        self.chunk_states.insert(
            coord,
            ChunkState::WantsMeshing {
                entity: needs_meshing_entity,
            },
        );
    }

    /// Called once a chunk starts meshing.
    pub fn mark_as_meshing(&mut self, coord: IVec3, meshing_task_entity: Entity) {
        self.chunk_states.insert(
            coord,
            ChunkState::Meshing {
                entity: meshing_task_entity,
            },
        );
    }

    /// Called once a chunk has finished meshing and is fully loaded.
    pub fn mark_as_loaded(&mut self, coord: IVec3, final_chunk_entity: Entity) {
        self.chunk_states.insert(
            coord,
            ChunkState::Loaded {
                entity: Some(final_chunk_entity),
            },
        );
    }

    /// Called once a chunk has finished meshing and is fully loaded.
    ///
    /// Passing no entity means the chunk is empty and does not need to be rendered.
    pub fn mark_as_loaded_but_empty(&mut self, coord: IVec3) {
        self.chunk_states
            .insert(coord, ChunkState::Loaded { entity: None });
    }

    /// Called when a chunk is unloaded, removing it from tracking.
    pub fn mark_as_unloaded(&mut self, coord: IVec3) {
        self.chunk_states.remove(&coord);
    }

    // INFO: -------------------------------
    //         util instance methods
    // -------------------------------------

    /// A help to iterate over all chunks needing meshing.
    ///
    /// Necessary to prevent throttling by only meshing a few
    /// chunks per frame/tick.
    pub fn iter_needs_meshing(&self) -> impl Iterator<Item = &IVec3> {
        self.chunk_states.iter().filter_map(|(coord, state)| {
            if matches!(state, ChunkState::WantsMeshing { .. }) {
                Some(coord)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over all *existing* neighbors of a chunk.
    ///
    /// A chunk neighbor is defined as ALL 26 other chunks in the 3x3
    /// cube that the chunk is the center of. Note that this only will
    /// yield neighbors currently tracked with an entity.
    pub fn iter_neighbors(&self, coord: IVec3) -> impl Iterator<Item = NeighborInfo> + '_ {
        NEIGHBOR_OFFSETS.into_iter().filter_map(move |offset| {
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

    // INFO: -----------------------------
    //         static util methods
    // -----------------------------------

    /// Determines if a coord is in bounds
    pub fn is_in_bounds(coord: IVec3) -> bool {
        let pos_y = coord.y;
        return (pos_y >= WORLD_MIN_Y_CHUNK) && (pos_y <= WORLD_MAX_Y_CHUNK);
    }
}

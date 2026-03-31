use crate::simulation_world::{
    block::BlockId, chunk::ChunkBlocksComponent, terrain::BiomeMapComponent,
};
use bevy::ecs::prelude::Component;

/// Contains all metadata calculated during generation.
#[derive(Component, Debug, Clone)]
pub struct ChunkMetadata {
    /// If true, all blocks in the chunk are identical.
    pub is_uniform: bool,
    /// If uniform, this is the ID. If mixed, this is None.
    /// Note: Used for optimization hints.
    pub uniform_block_id: Option<BlockId>,
    /// If true, the chunk contains at least one transparent block.
    pub contains_transparent: bool,
}

impl Default for ChunkMetadata {
    fn default() -> Self {
        Self {
            is_uniform: true,
            uniform_block_id: None,
            contains_transparent: false,
        }
    }
}

/// A struct to track metadata state during generation.
impl ChunkMetadata {
    pub fn new() -> Self {
        Self::default()
    }
}

// INFO: -----------------------
//         bundled types
// -----------------------------

pub struct GeneratedChunkComponentBundle {
    pub chunk_blocks: Option<ChunkBlocksComponent>,
    pub chunk_metadata: Option<ChunkMetadata>,
    pub biome_map: BiomeMapComponent,
}

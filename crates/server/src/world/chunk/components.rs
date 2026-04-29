use crate::world::terrain::BiomeMapComponent;
use shared::world::chunk::{ChunkBlocksComponent, ChunkMetadata};

pub struct GeneratedChunkComponentBundle {
    pub chunk_blocks: Option<ChunkBlocksComponent>,
    pub chunk_metadata: Option<ChunkMetadata>,
    pub biome_map: BiomeMapComponent,
}

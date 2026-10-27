use crate::world::terrain::BiomeMapComponent;
use bevy::prelude::*;
use common::world::chunk::{ChunkBlocksComponent, ChunkMetadata};

/// Marker component for chunks that are registered but haven't started generation.
#[derive(Component, Default, Debug)]
pub struct NeedsGenerating;

/// Marker component for chunks that are currently being generated.
#[derive(Component, Default, Debug)]
pub struct Generating;

/// Marker component for chunks that are fully generated and active in the world.
#[derive(Component, Default, Debug)]
pub struct ActiveChunk;

/// Marker component for chunks that are generated/loaded but contain no data (e.g., all air).
#[derive(Component, Default, Debug)]
pub struct EmptyChunk;

/// Marker component for chunks a player has edited.
///
/// These are never unloaded: there is no chunk persistence yet, so unloading one would
/// silently revert the player's edits.
#[derive(Component, Default, Debug)]
pub struct ModifiedChunk;

pub struct GeneratedChunkComponentBundle {
    pub chunk_blocks: Option<ChunkBlocksComponent>,
    pub chunk_metadata: Option<ChunkMetadata>,
    pub biome_map: BiomeMapComponent,
}

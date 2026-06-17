use bevy::prelude::{Component, Deref, DerefMut};
use shared::world::chunk::{ChunkColumnData, types::ChunkLod};

// INFO: --------------------------------
//         Biome gen climate data
// --------------------------------------

/// A representation of the climate data necessary for biome generation.
///
/// Includes all the parameters in terrain climate data, plus temperature and precipitation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClimateData {
    // below is used for both terrain and biome gen
    pub temperature: f32,
    pub precipitation: f32,
    // below is probably only needed for terraing gen
    pub continentalness: f32,
    pub erosion: f32,
    pub weirdness: f32,
}

// INFO: ----------------------------------
//         Terrain gen climate data
// ----------------------------------------

/// Stores the climate data (temperature, precipitation) for every COLUMN in a chunk.
#[derive(Component, Clone, Deref, DerefMut)]
pub struct ClimateMapComponent(pub ChunkColumnData<ClimateData>);

impl ClimateMapComponent {
    /// Creates a new climate map filled with 0.
    pub fn new_empty(lod: ChunkLod) -> Self {
        Self(ChunkColumnData::new_zeroed(lod))
    }
}

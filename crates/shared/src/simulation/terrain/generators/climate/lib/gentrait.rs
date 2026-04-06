use crate::simulation::{chunk::ChunkCoord, terrain::climate::ClimateMapComponent};

pub trait ClimateGenerator {
    fn generate(&self, chunk_coord: ChunkCoord) -> ClimateMapComponent;
}

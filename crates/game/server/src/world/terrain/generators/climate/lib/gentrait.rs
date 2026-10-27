use crate::world::terrain::climate::ClimateMapComponent;
use common::world::chunk::ChunkCoord;

pub trait ClimateGenerator {
    fn generate(&self, chunk_coord: ChunkCoord) -> ClimateMapComponent;
}

use shared::simulation::chunk::ChunkCoord;
use crate::simulation::terrain::climate::ClimateMapComponent;

pub trait ClimateGenerator {
    fn generate(&self, chunk_coord: ChunkCoord) -> ClimateMapComponent;
}

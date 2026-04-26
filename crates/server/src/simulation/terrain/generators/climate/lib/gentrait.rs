use crate::simulation::terrain::climate::ClimateMapComponent;
use shared::simulation::chunk::ChunkCoord;

pub trait ClimateGenerator {
    fn generate(&self, chunk_coord: ChunkCoord) -> ClimateMapComponent;
}

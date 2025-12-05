use crate::prelude::*;
use crate::simulation_world::{
    chunk::CHUNK_SIDE_LENGTH,
    terrain::climate::ClimateMapComponent,
    terrain::generators::shaping::{ChunkUniformity, ShapeResultBuilder, TerrainShaper},
};

use super::realistic_shaper::REALISTIC_SEA_LEVEL;

#[derive(Debug, Clone)]
pub struct SuperflatShaper {
    land_height: i32,
}

impl SuperflatShaper {
    pub fn new() -> Self {
        Self {
            land_height: REALISTIC_SEA_LEVEL as i32,
        }
    }
}

impl TerrainShaper for SuperflatShaper {
    fn name(&self) -> &str {
        "Superflat"
    }

    #[instrument(skip_all, fields(chunk = %coord))]
    fn determine_chunk_uniformity(&self, coord: IVec3) -> ChunkUniformity {
        let chunk_y_min = coord.y * CHUNK_SIDE_LENGTH as i32;
        let chunk_y_max = (coord.y + 1) * CHUNK_SIDE_LENGTH as i32 - 1;
        let world_surface_y = self.land_height;

        // if above highest, empty
        if chunk_y_min > world_surface_y {
            return ChunkUniformity::Empty;
        }

        // if below lowest, solid
        if chunk_y_max < world_surface_y {
            return ChunkUniformity::Solid;
        }

        // otherwise mixed (only the chunk layer at the surface height)
        ChunkUniformity::Mixed
    }

    #[instrument(skip_all)]
    fn shape_terrain_chunk(
        &self,
        _climate_map: &ClimateMapComponent,
        mut shaper: ShapeResultBuilder,
    ) -> ShapeResultBuilder {
        shaper.fill_from(|_local, world| world.y <= self.land_height);
        shaper
    }
}

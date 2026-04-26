use crate::prelude::*;
use crate::simulation::terrain::climate::ClimateMapComponent;
use crate::simulation::terrain::generators::shaping::{
    ChunkUniformity, ShapeResultBuilder, TerrainShaper,
};
use noise::{NoiseFn, Simplex};
use shared::simulation::chunk::CHUNK_SIDE_LENGTH;

/// Generates volumetric 3D terrain using Simplex noise.
#[derive(Debug, Clone)]
pub struct SimplexShaper {
    noise: Simplex,
    frequency: f64,
    /// Block determinance threshold for noise value, higher = more solid, lower = more empty
    threshold: f64,
    /// The bottom of the generated terrain. Below this is solid.
    floor_y: i32,
    /// The top of the generated terrain. Above this is air.
    ceiling_y: i32,
}

impl Default for SimplexShaper {
    fn default() -> Self {
        Self {
            noise: Simplex::new(1234),
            frequency: 0.02,
            threshold: 0.0, // -1 to 1 noise means 0 is about 50/50 air/solid
            floor_y: 0,
            ceiling_y: 256,
        }
    }
}

impl SimplexShaper {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TerrainShaper for SimplexShaper {
    fn name(&self) -> &str {
        "Simplex3D"
    }

    #[instrument(skip_all, fields(chunk = %coord))]
    fn determine_chunk_uniformity(&self, coord: IVec3) -> ChunkUniformity {
        let chunk_y_min = coord.y * CHUNK_SIDE_LENGTH as i32;
        let chunk_y_max = (coord.y + 1) * CHUNK_SIDE_LENGTH as i32 - 1;

        if chunk_y_min > self.ceiling_y {
            return ChunkUniformity::Empty;
        }

        if chunk_y_max < self.floor_y {
            return ChunkUniformity::Solid;
        }

        ChunkUniformity::Mixed
    }

    #[instrument(skip_all)]
    fn shape_terrain_chunk(
        &self,
        _climate_map: &ClimateMapComponent,
        mut shape_builder: ShapeResultBuilder,
    ) -> ShapeResultBuilder {
        let threshold = self.threshold;

        shape_builder.fill_from(|_local, world| {
            if world.y > self.ceiling_y {
                return false;
            }
            if world.y < self.floor_y {
                return true;
            }

            let p = [
                world.x as f64 * self.frequency,
                world.y as f64 * self.frequency,
                world.z as f64 * self.frequency,
            ];

            let val = self.noise.get(p);

            val > threshold
        });

        shape_builder
    }
}

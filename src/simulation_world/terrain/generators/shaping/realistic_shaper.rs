use crate::prelude::*;
use crate::simulation_world::chunk::CHUNK_SIDE_LENGTH;
use crate::simulation_world::terrain::climate::ClimateMapComponent;
use crate::simulation_world::terrain::generators::shaping::{
    ChunkUniformity, ShapeResultBuilder, TerrainShaper,
};
use noise::{NoiseFn, Simplex};

pub const REALISTIC_SEA_LEVEL: f64 = 64.0;

/// Generates volumetric 3D terrain using Climate-driven Density Functions.
#[derive(Debug, Clone)]
pub struct RealisticShaper {
    noise: Simplex,
    frequency: f64,
    floor_y: i32,
    ceiling_y: i32,
    sea_level: f64,
}

impl RealisticShaper {
    pub fn new() -> Self {
        Self {
            noise: Simplex::new(1234),
            frequency: 0.015,
            floor_y: 0,
            ceiling_y: 256,
            sea_level: REALISTIC_SEA_LEVEL,
        }
    }

    fn map_range(val: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
        (val - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
    }
}

impl TerrainShaper for RealisticShaper {
    fn name(&self) -> &str {
        "ClimateRealistic"
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
        climate_map: &ClimateMapComponent,
        mut shape_builder: ShapeResultBuilder,
    ) -> ShapeResultBuilder {
        shape_builder.fill_from(|local, world| {
            if world.y > self.ceiling_y {
                return false;
            }
            if world.y < self.floor_y {
                return true;
            }

            let climate = climate_map.get_data_unchecked(local.x as usize, local.z as usize);

            let target_height = Self::map_range(
                climate.continentalness as f64,
                -1.0,
                1.0,
                self.sea_level - 40.0,
                self.sea_level + 140.0,
            );

            let noise_amplitude = Self::map_range(climate.erosion as f64, -1.0, 1.0, 1.5, 0.1);
            let weirdness_factor = climate.weirdness as f64 * 0.2;

            let mut density = (target_height - world.y as f64) / 40.0;

            let noise_sample = self.noise.get([
                world.x as f64 * self.frequency,
                world.y as f64 * self.frequency * 1.5,
                world.z as f64 * self.frequency,
            ]);

            density += (noise_sample + weirdness_factor) * noise_amplitude;

            density > 0.0
        });

        shape_builder
    }
}

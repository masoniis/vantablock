use super::realistic_shaper::REALISTIC_SEA_LEVEL;
use crate::prelude::*;
use crate::simulation_world::chunk::CHUNK_SIDE_LENGTH;
use crate::simulation_world::terrain::climate::ClimateMapComponent;
use crate::simulation_world::terrain::generators::shaping::{
    ChunkUniformity, ShapeResultBuilder, TerrainShaper,
};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

#[derive(Debug, Clone)]
pub struct NoisyShaper {
    noise: Fbm<Perlin>,
    base_height: i32,
    amplitude: f64,
}

impl NoisyShaper {
    pub fn new() -> Self {
        let mut noise = Fbm::new(1234);
        noise = noise.set_frequency(0.01);
        noise = noise.set_octaves(5);
        noise = noise.set_lacunarity(2.2);
        noise = noise.set_persistence(0.55);

        Self {
            noise,
            base_height: REALISTIC_SEA_LEVEL as i32,
            amplitude: 24.0,
        }
    }
}

impl TerrainShaper for NoisyShaper {
    fn name(&self) -> &str {
        "NoisyAmplitude"
    }

    #[instrument(skip_all, fields(chunk = %coord))]
    fn determine_chunk_uniformity(&self, coord: IVec3) -> ChunkUniformity {
        let chunk_y_min = coord.y * CHUNK_SIDE_LENGTH as i32;
        let chunk_y_max = (coord.y + 1) * CHUNK_SIDE_LENGTH as i32 - 1;

        let max_variation = self.amplitude;
        let max_possible_y = (self.base_height as f64 + max_variation).round() as i32;

        // if above max y, all empty
        if chunk_y_min > max_possible_y {
            return ChunkUniformity::Empty;
        }

        let min_possible_y = (self.base_height as f64 - max_variation).round() as i32;
        let effective_terrain_floor = min_possible_y.max(1);

        // if below sin variation, all solid
        if chunk_y_max < effective_terrain_floor {
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
        let base = self.base_height as f64;
        let amp = self.amplitude;

        shape_builder.fill_from(|_local, world| {
            let p = [world.x as f64, world.z as f64];
            let noise_val = self.noise.get(p);
            let surface_y = (base + (noise_val * amp)).round() as i32;

            world.y < surface_y
        });

        shape_builder
    }
}

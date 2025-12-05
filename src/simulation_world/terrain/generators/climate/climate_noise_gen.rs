use super::climate_buffer_pool::CLIMATE_BUFFERS;
use crate::{
    prelude::*,
    simulation_world::{
        chunk::{ChunkCoord, ChunkLod},
        terrain::{
            climate::{ClimateData, ClimateMapComponent},
            ClimateGenerator,
        },
    },
};
use bevy_ecs::resource::Resource;
use noise::{Fbm, MultiFractal, NoiseFn, OpenSimplex};

/// The number of octaves used in the noise functions.
const CLIMATE_NOISE_OCTAVES: usize = 2;

/// Helper function to create a standard FBM noise function
fn create_noise_fn(seed: u32, octaves: usize) -> Fbm<OpenSimplex> {
    Fbm::new(seed)
        .set_octaves(octaves)
        .set_frequency(0.0033)
        .set_lacunarity(2.0)
        .set_persistence(0.5)
}

#[derive(Resource, Clone)]
pub struct ClimateNoiseGenerator {
    // BIOME-ONLY parameters
    temperature_noise: Fbm<OpenSimplex>,
    precipitation_noise: Fbm<OpenSimplex>,

    // BIOME + TERRAIN-GEN parameters
    continental_noise: Fbm<OpenSimplex>,
    erosion_noise: Fbm<OpenSimplex>,
    weirdness_noise: Fbm<OpenSimplex>,
}

impl ClimateNoiseGenerator {
    #[instrument(skip_all)]
    pub fn new(seed: u32) -> Self {
        Self {
            temperature_noise: create_noise_fn(seed, CLIMATE_NOISE_OCTAVES),
            precipitation_noise: create_noise_fn(seed.wrapping_add(1), CLIMATE_NOISE_OCTAVES),

            continental_noise: create_noise_fn(seed.wrapping_add(2), CLIMATE_NOISE_OCTAVES),
            weirdness_noise: create_noise_fn(seed.wrapping_add(3), CLIMATE_NOISE_OCTAVES),
            erosion_noise: create_noise_fn(seed.wrapping_add(4), CLIMATE_NOISE_OCTAVES),
        }
    }

    /// Calculates all 5 climate values for a single world-space block coordinate.
    #[instrument(skip_all)]
    pub fn get_climate_at(&self, world_x: i32, world_z: i32) -> ClimateData {
        let sample_2d = [world_x as f64, world_z as f64];

        // BIOME-ONLY parameters
        let temperature = self.temperature_noise.get(sample_2d) as f32;
        let precipitation = self.precipitation_noise.get(sample_2d) as f32;

        // BIOME + TERRAIN-GEN parameters
        let continentalness = self.continental_noise.get(sample_2d) as f32;
        let erosion = self.erosion_noise.get(sample_2d) as f32;
        let weirdness = self.weirdness_noise.get(sample_2d) as f32;

        ClimateData {
            temperature,
            precipitation,
            continentalness,
            erosion,
            weirdness,
        }
    }

    /// Calculates all 5 climate values in a batch for a whole buffer efficiently.
    fn generate_single_map(
        &self,
        coords: &[[f64; 2]],
        target_buffer: &mut [f32],
        noise_fn: &Fbm<OpenSimplex>,
    ) {
        for (i, point) in coords.iter().enumerate() {
            let raw = noise_fn.get(*point);
            target_buffer[i] = raw as f32;
        }
    }

    /// Orchestrates the filling of all 5 climate buffers
    fn orchestrate_fill(
        &self,
        buffers: &mut super::climate_buffer_pool::ClimateBufferPool,
        base_x: i32,
        base_z: i32,
        size: usize,
    ) {
        let area = size * size;

        // calculate coordinates
        let mut coords = vec![[0.0; 2]; area];
        for x in 0..size {
            for z in 0..size {
                let idx = x * size + z;
                let wx = (base_x + x as i32) as f64;
                let wz = (base_z + z as i32) as f64;
                coords[idx] = [wx, wz];
            }
        }

        self.generate_single_map(
            &coords,
            buffers.temperature.as_mut_slice(),
            &self.temperature_noise,
        );
        self.generate_single_map(
            &coords,
            buffers.precipitation.as_mut_slice(),
            &self.precipitation_noise,
        );
        self.generate_single_map(
            &coords,
            buffers.continentalness.as_mut_slice(),
            &self.continental_noise,
        );
        self.generate_single_map(&coords, buffers.erosion.as_mut_slice(), &self.erosion_noise);
        self.generate_single_map(
            &coords,
            buffers.weirdness.as_mut_slice(),
            &self.weirdness_noise,
        );
    }
}

impl ClimateGenerator for ClimateNoiseGenerator {
    fn generate(&self, chunk_coord: ChunkCoord) -> ClimateMapComponent {
        let lod = ChunkLod(0);
        let mut climate_map = ClimateMapComponent::new_empty(lod);
        let size = climate_map.size();
        let base_pos = chunk_coord.as_world_pos();

        CLIMATE_BUFFERS.with(|cell| {
            let mut buffers = cell.borrow_mut();
            buffers.prepare(size);

            // fill all buffers
            self.orchestrate_fill(&mut *buffers, base_pos.x, base_pos.z, size);

            let mut writer = climate_map.get_data_writer();
            let area = size * size;

            // read results and populate into ClimateData
            let temp = buffers.temperature.as_slice();
            let precip = buffers.precipitation.as_slice();
            let cont = buffers.continentalness.as_slice();
            let erosion = buffers.erosion.as_slice();
            let weird = buffers.weirdness.as_slice();

            for i in 0..area {
                let climate_data = ClimateData {
                    temperature: temp[i],
                    precipitation: precip[i],
                    continentalness: cont[i],
                    erosion: erosion[i],
                    weirdness: weird[i],
                };

                writer.set_at_index(i, climate_data);
            }
        });

        climate_map
    }
}

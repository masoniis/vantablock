use super::climate_buffer_pool::CLIMATE_BUFFERS;
use crate::simulation::terrain::{
    ClimateGenerator,
    climate::{ClimateData, ClimateMapComponent},
};
use bevy::ecs::prelude::Resource;
use noise::MultiFractal;
use shared::simulation::chunk::{ChunkCoord, ChunkLod};

#[derive(Resource)]
pub struct ClimateNoiseGenerator {
    temperature_noise: noise::Fbm<noise::OpenSimplex>,
    precipitation_noise: noise::Fbm<noise::OpenSimplex>,
    continental_noise: noise::Fbm<noise::OpenSimplex>,
    erosion_noise: noise::Fbm<noise::OpenSimplex>,
    weirdness_noise: noise::Fbm<noise::OpenSimplex>,
}

impl ClimateNoiseGenerator {
    pub fn new(seed: u32) -> Self {
        use noise::{Fbm, OpenSimplex};

        // TODO: Tune noise parameters
        Self {
            temperature_noise: Fbm::<OpenSimplex>::new(seed).set_frequency(0.005),
            precipitation_noise: Fbm::<OpenSimplex>::new(seed + 1).set_frequency(0.005),
            continental_noise: Fbm::<OpenSimplex>::new(seed + 2).set_frequency(0.003),
            erosion_noise: Fbm::<OpenSimplex>::new(seed + 3).set_frequency(0.004),
            weirdness_noise: Fbm::<OpenSimplex>::new(seed + 4).set_frequency(0.01),
        }
    }

    /// Fills a specific float buffer with values from a noise function
    fn generate_single_map(
        &self,
        coords: &[[f64; 2]],
        target_buffer: &mut [f32],
        noise_fn: &impl noise::NoiseFn<f64, 2>,
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

        CLIMATE_BUFFERS.with(|pool| {
            let mut buffers = pool.borrow_mut();
            buffers.prepare(size);

            // fill all buffers
            self.orchestrate_fill(&mut buffers, base_pos.x, base_pos.z, size);

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

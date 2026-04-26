use crate::prelude::*;
use crate::simulation::terrain::BiomeMapComponent;
use crate::simulation::terrain::climate::ClimateMapComponent;
use shared::simulation::{
    biome::{BiomeId, BiomeRegistryResource},
    chunk::{ChunkCoord, VolumeDataWriter},
};
use std::fmt::Debug;

// INFO: -------------------------
//         biome generator
// -------------------------------

/// A trait for just filling the biome map
pub trait BiomeGenerator: Send + Sync + Debug {
    fn generate_biome_chunk(
        &self,
        builder: BiomeResultBuilder,
        climate_map: &ClimateMapComponent,
        biome_registry: &BiomeRegistryResource,
    ) -> BiomeResultBuilder;
}

/// A writer for updating biome data.
pub struct BiomeWriter<'a> {
    biome_writer: VolumeDataWriter<'a, BiomeId>,
    pub chunk_coord: ChunkCoord,
}

impl<'a> BiomeWriter<'a> {
    #[inline(always)]
    pub fn set_biome(&mut self, x: usize, y: usize, z: usize, biome_id: BiomeId) {
        self.biome_writer.set_data(x, y, z, biome_id);
    }
}

pub struct BiomeResultBuilder {
    biome_map: BiomeMapComponent,
    chunk_coord: ChunkCoord,
}

impl BiomeResultBuilder {
    pub fn new(biome_map: BiomeMapComponent, chunk_coord: ChunkCoord) -> Self {
        Self {
            biome_map,
            chunk_coord,
        }
    }

    /// Finish biome generation and take ownership of the inner components.
    pub fn finish(self) -> BiomeMapComponent {
        self.biome_map
    }

    /// Returns the size of the chunk.
    pub fn size(&self) -> usize {
        self.biome_map.size()
    }

    /// Opens a manual edit scope for arbitrary writes.
    #[inline(always)]
    pub fn edit_arbitrary(&mut self, mut f: impl FnMut(&mut BiomeWriter)) {
        let biome_writer = self.biome_map.get_data_writer();
        let mut writer = BiomeWriter {
            biome_writer,
            chunk_coord: self.chunk_coord.clone(),
        };
        f(&mut writer);
    }

    /// Runs an optimally structured loop (X, Z, Y) to fill biomes based on the closure.
    ///
    /// The closure should return a `BiomeId`.
    #[inline(always)]
    pub fn fill_from(&mut self, f: impl Fn(IVec3, IVec3) -> BiomeId) {
        let size = self.biome_map.size() as i32;
        let base_world = self.chunk_coord.as_world_pos();
        let step = 1 << self.biome_map.lod().0;

        self.edit_arbitrary(|writer| {
            let base_x = base_world.x;
            let base_y = base_world.y;
            let base_z = base_world.z;

            for x in 0..size {
                let world_x = base_x + (x * step);
                for z in 0..size {
                    let world_z = base_z + (z * step);
                    for y in 0..size {
                        let world_y = base_y + (y * step);

                        let local = IVec3::new(x, y, z);
                        let world = IVec3::new(world_x, world_y, world_z);

                        let biome_id = f(local, world);
                        writer.set_biome(x as usize, y as usize, z as usize, biome_id);
                    }
                }
            }
        });
    }
}

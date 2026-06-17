use crate::prelude::*;
use crate::world::terrain::{
    climate::ClimateMapComponent,
    generators::biome::{BiomeGenerator, BiomeResultBuilder},
};
use shared::world::biome::BiomeRegistryResource;

// A default implementation
#[derive(Debug, Default)]
pub struct BasicBiomeGenerator;

impl BiomeGenerator for BasicBiomeGenerator {
    #[instrument(skip_all)]
    fn generate_biome_chunk(
        &self,
        mut builder: BiomeResultBuilder,
        climate_map: &ClimateMapComponent,
        biome_registry: &BiomeRegistryResource,
    ) -> BiomeResultBuilder {
        let plains_id = biome_registry.get_biome_id_or_default("plains");
        let ocean_id = biome_registry.get_biome_id_or_default("ocean");

        let size = builder.size();
        builder.edit_arbitrary(|writer| {
            for x in 0..size {
                for z in 0..size {
                    // noise data
                    let climate_data_for_column = climate_map.get_data_unchecked(x, z);

                    // set map for writer
                    for y in 0..size {
                        let temperature = climate_data_for_column.temperature;
                        if temperature >= 0.5 {
                            writer.set_biome(x, y, z, plains_id);
                        } else {
                            writer.set_biome(x, y, z, ocean_id);
                        }
                    }
                }
            }
        });

        builder
    }
}

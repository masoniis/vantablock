use crate::prelude::*;
use crate::simulation_world::{
    biome::BiomeRegistryResource,
    terrain::{
        climate::{ClimateData, ClimateMapComponent},
        generators::biome::{BiomeGenerator, BiomeResultBuilder},
    },
};

#[derive(Debug, Default)]
pub struct MultiNoiseBiomeGenerator;

struct BiomeHandles {
    ocean: u8,
    deep_ocean: u8,
    beach: u8,

    // Cold
    snowy_plains: u8,
    ice_spikes: u8,

    // Temperate
    plains: u8,
    forest: u8,
    swamp: u8,

    // Hot
    desert: u8,
    jungle: u8,
    savanna: u8,
    badlands: u8,

    // High Altitude
    stony_peaks: u8,
    snowy_peaks: u8,
}

impl BiomeGenerator for MultiNoiseBiomeGenerator {
    #[instrument(skip_all)]
    fn generate_biome_chunk(
        &self,
        mut builder: BiomeResultBuilder,
        climate_map: &ClimateMapComponent,
        biome_registry: &BiomeRegistryResource,
    ) -> BiomeResultBuilder {
        let handles = BiomeHandles {
            ocean: biome_registry.get_biome_id_or_default("ocean"),
            deep_ocean: biome_registry.get_biome_id_or_default("deep_ocean"),
            beach: biome_registry.get_biome_id_or_default("beach"),
            snowy_plains: biome_registry.get_biome_id_or_default("snowy_plains"),
            ice_spikes: biome_registry.get_biome_id_or_default("ice_spikes"),
            plains: biome_registry.get_biome_id_or_default("plains"),
            forest: biome_registry.get_biome_id_or_default("forest"),
            swamp: biome_registry.get_biome_id_or_default("swamp"),
            desert: biome_registry.get_biome_id_or_default("desert"),
            jungle: biome_registry.get_biome_id_or_default("jungle"),
            savanna: biome_registry.get_biome_id_or_default("savanna"),
            badlands: biome_registry.get_biome_id_or_default("badlands"),
            stony_peaks: biome_registry.get_biome_id_or_default("stony_peaks"),
            snowy_peaks: biome_registry.get_biome_id_or_default("snowy_peaks"),
        };

        let size = builder.size();

        builder.edit_arbitrary(|writer| {
            for x in 0..size {
                for z in 0..size {
                    let climate = climate_map.get_data_unchecked(x, z);
                    let biome_id = resolve_biome(&handles, &climate);

                    // fill
                    for y in 0..size {
                        writer.set_biome(x, y, z, biome_id);
                    }
                }
            }
        });

        builder
    }
}

/// logic for mapping climate noise to a specific Biome ID.
///
/// Assuems -1 to 1 on the climate noise gen
fn resolve_biome(h: &BiomeHandles, c: &ClimateData) -> u8 {
    // INFO: -----------------------------------------
    //         continentalness (ocean vs land)
    // -----------------------------------------------

    if c.continentalness < -0.6 {
        return h.deep_ocean;
    }
    if c.continentalness < -0.15 {
        return h.ocean;
    }
    if c.continentalness < -0.05 {
        return h.beach;
    }

    // INFO: -----------------------------
    //         erosion (mountains)
    // -----------------------------------

    if c.erosion < -0.375 || (c.erosion < -0.25 && c.weirdness > 0.5) {
        if c.temperature < -0.2 {
            return h.snowy_peaks;
        } else {
            return h.stony_peaks;
        }
    }

    // INFO: -----------------------
    //         temp/humidity
    // -----------------------------

    if c.temperature < -0.2 {
        // cold
        if c.weirdness > 0.6 {
            h.ice_spikes
        } else {
            h.snowy_plains
        }
    } else if c.temperature < 0.2 {
        // temperate
        if c.precipitation < -0.1 {
            // dry
            h.plains
        } else if c.precipitation > 0.1 {
            // wet
            if c.weirdness > 0.4 {
                h.swamp
            } else {
                h.forest
            }
        } else {
            // average temp
            if c.weirdness < 0.0 {
                h.forest
            } else {
                h.plains
            }
        }
    } else {
        // hot
        if c.precipitation < -0.1 {
            if c.weirdness > 0.2 {
                h.badlands
            } else {
                h.desert
            }
        } else if c.precipitation > 0.1 {
            // wet
            h.jungle
        } else {
            // semi-dry
            h.savanna
        }
    }
}

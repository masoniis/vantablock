use crate::prelude::*;
use crate::simulation_world::biome::BiomeRegistryResource;
use crate::simulation_world::block::BlockRegistryResource;
use crate::simulation_world::terrain::generators::painting::{PaintResultBuilder, TerrainPainter};
use crate::simulation_world::terrain::shaping::realistic_shaper::REALISTIC_SEA_LEVEL;
use crate::simulation_world::terrain::BiomeMapComponent;

#[derive(Debug, Clone)]
pub struct SimpleSurfacePainter;

impl SimpleSurfacePainter {
    pub fn new() -> Self {
        Self
    }
}

impl TerrainPainter for SimpleSurfacePainter {
    #[instrument(skip_all)]
    fn paint_terrain_chunk(
        &self,
        mut painter: PaintResultBuilder,
        biome_map: &BiomeMapComponent,
        block_registry: &BlockRegistryResource,
        biome_registry: &BiomeRegistryResource,
    ) -> PaintResultBuilder {
        let air_id = block_registry.get_block_id_by_name("air").unwrap();
        let water_id = block_registry.get_block_id_by_name("water").unwrap();
        let stone_id = block_registry.get_block_id_by_name("stone").unwrap();

        let size = painter.size();
        let base_y = painter.chunk_coord.as_world_pos().y;

        painter.edit_arbitrary(|writer| {
            for x in 0..size {
                for z in 0..size {
                    // iterate backwards to find first surface block
                    for y in (0..size).rev() {
                        if writer.get_block(x, y, z) != air_id {
                            let world_y = base_y + y as i32;

                            // biome data for column
                            let biome_id = biome_map.get_data_unchecked(x, y, z);
                            let biome_def = biome_registry.get(biome_id);

                            let surface_id = block_registry
                                .get_block_id_by_name(&biome_def.terrain.surface_material)
                                .unwrap_or(stone_id);
                            let subsurface_id = block_registry
                                .get_block_id_by_name(&biome_def.terrain.subsurface_material)
                                .unwrap_or(stone_id);

                            let actual_surface_id = if world_y < REALISTIC_SEA_LEVEL as i32 {
                                subsurface_id
                            } else {
                                surface_id
                            };

                            // apply surface and subsurface blocks (but dont do water,
                            // since that relies on just being filled below sea level)
                            if actual_surface_id != water_id {
                                writer.set_block(x, y, z, actual_surface_id);
                                for i in 1..=3 {
                                    if y >= i {
                                        let sy = y - i;
                                        if writer.get_block(x, sy, z) != air_id {
                                            writer.set_block(x, sy, z, subsurface_id);
                                        }
                                    }
                                }
                            }

                            break;
                        }
                    }

                    // water replaces air below sea level
                    for y in 0..size {
                        let world_y = base_y + y as i32;
                        if world_y < REALISTIC_SEA_LEVEL as i32 {
                            if writer.get_block(x, y, z) == air_id {
                                writer.set_block(x, y, z, water_id);
                            }
                        } else {
                            // above sea level, we can stop checking water
                            break;
                        }
                    }
                }
            }
        });

        painter
    }
}

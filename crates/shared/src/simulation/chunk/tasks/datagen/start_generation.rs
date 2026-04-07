use crate::prelude::*;
use crate::simulation::{
    biome::BiomeRegistryResource,
    block::BlockRegistryResource,
    chunk::{
        ChunkBlocksComponent, ChunkCoord, ChunkGenerationTaskComponent, ChunkState,
        ChunkStateManager, NeedsGenerating, components::GeneratedChunkComponentBundle,
    },
    terrain::{
        ActiveBiomeGenerator, ActiveClimateGenerator, ActiveTerrainGenerator, ActiveTerrainPainter,
        BiomeMapComponent,
        generators::{
            biome::BiomeResultBuilder, painting::PaintResultBuilder,
            shaping::ChunkUniformity, shaping::ShapeResultBuilder,
        },
    },
};
use bevy::ecs::prelude::*;
use crossbeam::channel::unbounded;

/// Queries for entities needing generation and starts a limited number per frame.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn start_pending_generation_tasks_system(
    // Input
    mut pending_chunks_query: Query<
        (Entity, &NeedsGenerating, &ChunkCoord),
        Without<ChunkGenerationTaskComponent>,
    >,

    // Output/Resources
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkStateManager>,
    block_registry: Res<BlockRegistryResource>,
    biome_registry: Res<BiomeRegistryResource>,
    biome_generator: Res<ActiveBiomeGenerator>,
    terrain_generator: Res<ActiveTerrainGenerator>,
    terrain_painter: Res<ActiveTerrainPainter>,
    climate_generator: Res<ActiveClimateGenerator>,
) {
    for (entity, needs_generating, coord) in pending_chunks_query.iter_mut() {
        // check for cancellation
        match chunk_manager.get_state(coord.pos) {
            Some(ChunkState::NeedsGenerating {
                entity: state_entity,
            }) if state_entity == entity => {
                // state is correct, proceed to start generation
            }
            _ => {
                debug!(
                    target : "chunk_loading",
                    "Entity {:?} NeedsGenerating for chunk {} found, but manager state ({:?}) doesn't match NeedsGenerating({:?}). Assuming cancelled/stale.",
                    entity, coord, chunk_manager.get_state(coord.pos), entity
                );
                continue;
            }
        }

        let lod = needs_generating.lod;

        // check if the chunk is empty according to the terrain generator
        match terrain_generator.0.determine_chunk_uniformity(coord.pos) {
            ChunkUniformity::Empty => {
                trace!(
                    target: "chunk_loading",
                    "Chunk {} is empty according to terrain generator. Skipping generation.",
                    coord
                );
                commands.entity(entity).despawn();
                chunk_manager.mark_as_loaded_but_empty(coord.pos);
                continue;
            }
            ChunkUniformity::Solid => {
                let chunk_blocks = ChunkBlocksComponent::new_uniform_solid(lod);

                let bundle = GeneratedChunkComponentBundle {
                    chunk_blocks: Some(chunk_blocks),
                    chunk_metadata: None,
                    biome_map: BiomeMapComponent::new_empty(lod),
                };

                // instant completed task
                let (sender, receiver) = unbounded();
                let _ = sender.send(bundle);

                commands
                    .entity(entity)
                    .insert(ChunkGenerationTaskComponent { receiver })
                    .remove::<NeedsGenerating>();

                chunk_manager.mark_as_generating(coord.pos, entity);
                continue;
            }
            _ => {}
        }

        // start the generation thread task if not
        let (sender, receiver) = unbounded();

        let blocks_clone = block_registry.clone();
        let biomes_clone = biome_registry.clone();
        let terrain_gen = terrain_generator.0.clone();
        let biome_gen = biome_generator.0.clone();
        let terrain_paint = terrain_painter.0.clone();
        let climate_gen = climate_generator.0.clone();
        let coord_clone = coord.clone();

        rayon::spawn(move || {
            // for testing, override the chunk lod
            // let mut lod = ChunkLod(0);
            // if coord_clone.z >= 5 && (coord_clone.x == 0 || coord_clone.x == 1) {
            //     lod = ChunkLod(3);
            // }

            // INFO: biome gen
            let climate_map = climate_gen.generate(coord_clone.clone());
            let biome_map = BiomeMapComponent::new_empty(lod);
            let biome_builder = BiomeResultBuilder::new(biome_map, coord_clone.clone());
            let biome_map = biome_gen
                .generate_biome_chunk(biome_builder, &climate_map, &biomes_clone)
                .finish();

            // INFO: shaping
            let chunk_blocks = ChunkBlocksComponent::new_uniform_empty(lod);
            let shaper = ShapeResultBuilder::new(chunk_blocks, coord_clone.clone());
            let shaped_chunk_blocks = terrain_gen
                .shape_terrain_chunk(&climate_map, shaper)
                .finish();

            // INFO: painting
            let painter_builder = PaintResultBuilder::new(
                shaped_chunk_blocks,
                coord_clone.clone(),
                blocks_clone.clone(),
            );
            let (painted_chunk_blocks, chunk_metadata) = terrain_paint
                .paint_terrain_chunk(painter_builder, &biome_map, &blocks_clone, &biomes_clone)
                .finish();

            trace!(
                target: "chunk_loading",
                "Finished generation for chunk {}.",
                coord_clone
            );

            let bundle = GeneratedChunkComponentBundle {
                chunk_blocks: Some(painted_chunk_blocks),
                chunk_metadata: Some(chunk_metadata),
                biome_map,
            };
            let _ = sender.send(bundle);
        });

        trace!(
            target: "chunk_loading",
            "Spawned generation task for chunk {}.",
            coord
        );

        commands
            .entity(entity)
            .insert(ChunkGenerationTaskComponent { receiver })
            .remove::<NeedsGenerating>();

        chunk_manager.mark_as_generating(coord.pos, entity);
    }
}

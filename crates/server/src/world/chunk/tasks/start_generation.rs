use crate::{
    prelude::*,
    world::{
        chunk::components::{
            ActiveChunk, GeneratedChunkComponentBundle, Generating, NeedsGenerating,
        },
        chunk::tasks::components::ChunkGenerationTaskComponent,
        terrain::{
            ActiveBiomeGenerator, ActiveClimateGenerator, ActiveTerrainGenerator,
            ActiveTerrainPainter, BiomeMapComponent,
            generators::{
                biome::BiomeResultBuilder, painting::PaintResultBuilder, shaping::ChunkUniformity,
                shaping::ShapeResultBuilder,
            },
        },
    },
};
use bevy::{ecs::prelude::*, tasks::AsyncComputeTaskPool};
use crossbeam::channel::unbounded;
use shared::world::{
    biome::BiomeRegistryResource,
    block::BlockRegistry,
    chunk::{ChunkBlocksComponent, ChunkCoord, ChunkLod},
};

/// Queries for entities needing generation and starts a limited number per frame.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn start_pending_generation_tasks_system(
    // input
    mut pending_chunks_query: Query<
        (Entity, &ChunkLod, &ChunkCoord),
        (With<NeedsGenerating>, Without<ChunkGenerationTaskComponent>),
    >,

    // output
    mut commands: Commands,
    block_registry: Res<BlockRegistry>,
    biome_registry: Res<BiomeRegistryResource>,
    biome_generator: Res<ActiveBiomeGenerator>,
    terrain_generator: Res<ActiveTerrainGenerator>,
    terrain_painter: Res<ActiveTerrainPainter>,
    climate_generator: Res<ActiveClimateGenerator>,
) {
    const MAX_CHUNKS_PER_FRAME: usize = 4;

    for (entity, lod_comp, coord) in pending_chunks_query.iter_mut().take(MAX_CHUNKS_PER_FRAME) {
        let lod = *lod_comp;

        // check if the chunk is empty according to the terrain generator
        match terrain_generator.0.determine_chunk_uniformity(coord.pos) {
            ChunkUniformity::Empty => {
                let chunk_blocks = ChunkBlocksComponent::new_uniform_empty(lod);

                commands
                    .entity(entity)
                    .insert((chunk_blocks, ActiveChunk))
                    .remove::<NeedsGenerating>();
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
                    .insert((ChunkGenerationTaskComponent { receiver }, Generating))
                    .remove::<NeedsGenerating>();
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

        AsyncComputeTaskPool::get()
            .spawn(async move {
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
            })
            .detach();

        trace!(
            target: "chunk_loading",
            "Spawned generation task for chunk {}.",
            coord
        );

        commands
            .entity(entity)
            .insert((ChunkGenerationTaskComponent { receiver }, Generating))
            .remove::<NeedsGenerating>();
    }
}

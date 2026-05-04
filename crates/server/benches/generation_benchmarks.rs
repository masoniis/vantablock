use bevy::ecs::prelude::World;
use criterion::{Criterion, criterion_group, criterion_main};
use server::{
    prelude::*,
    world::terrain::{
        BasicBiomeGenerator, BiomeGenerator, BiomeMapComponent, BiomeResultBuilder,
        ClimateGenerator, ClimateNoiseGenerator, PaintResultBuilder, ShapeResultBuilder,
        SimpleSurfacePainter, SinwaveShaper, TerrainPainter, TerrainShaper,
    },
};
use shared::{
    lifecycle::PersistentPathsResource,
    world::{
        biome::biome_registry::BiomeRegistryResource,
        block::BlockRegistry,
        chunk::{
            components::{ChunkBlocksComponent, ChunkCoord},
            types::ChunkLod,
        },
    },
};
use utils::PersistentPaths;

const CLIMATE_NOISE_SEED: u32 = 42;

/// Each bench in this benchmark builds off the previous (conceptually speaking).
fn bench_chunk_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Chunk generation");

    // INFO: ---------------
    //         setup
    // ---------------------

    let persistent_paths = PersistentPaths::resolve_client();
    let block_registry = BlockRegistry::load_from_disk(&persistent_paths);

    let mut world = World::new();
    world.insert_resource(PersistentPathsResource(persistent_paths.clone()));
    world.insert_resource(block_registry.clone());
    world.insert_resource(BiomeRegistryResource::load_from_disk(&persistent_paths));

    let biome_registry = world.resource::<BiomeRegistryResource>().clone();

    let origin_chunk_coord = ChunkCoord {
        pos: IVec3::new(0, 0, 0),
    };

    // INFO: --------------------------
    //         climate benching
    // --------------------------------

    let climate_noise_generator = ClimateNoiseGenerator::new(CLIMATE_NOISE_SEED);
    group.bench_function("climate_noise", |b| {
        b.iter(|| {
            climate_noise_generator.generate(origin_chunk_coord.clone());
        })
    });

    // INFO: ------------------------
    //         biome benching
    // ------------------------------

    let biome_generator = BasicBiomeGenerator;
    let origin_noise = climate_noise_generator.generate(origin_chunk_coord.clone());

    group.bench_function("biome_mapping", |b| {
        b.iter(|| {
            // setup
            let biome_map = BiomeMapComponent::new_empty(ChunkLod(0));
            let builder = BiomeResultBuilder::new(biome_map, origin_chunk_coord.clone());

            // chunk gen
            biome_generator.generate_biome_chunk(builder, &origin_noise, &biome_registry);
        })
    });

    // INFO: --------------------------------
    //         sinwave shape benching
    // --------------------------------------

    let sinwave_shaper = SinwaveShaper::default();
    group.bench_function("sinwave_shaping", |b| {
        b.iter(|| {
            let clim_map = climate_noise_generator.generate(origin_chunk_coord.clone());
            let chunk_blocks = ChunkBlocksComponent::new_uniform_empty(ChunkLod(0));
            let shaper = ShapeResultBuilder::new(chunk_blocks, origin_chunk_coord.clone());

            sinwave_shaper.shape_terrain_chunk(&clim_map, shaper)
        })
    });

    // INFO: ------------------------
    //         paint benching
    // ------------------------------

    let surface_painter = SimpleSurfacePainter::default();
    group.bench_function("painting", |b| {
        b.iter(|| {
            let biome_map = BiomeMapComponent::new_empty(ChunkLod(0));

            let blocks = ChunkBlocksComponent::new_uniform_solid(ChunkLod(0));
            let painter =
                PaintResultBuilder::new(blocks, origin_chunk_coord.clone(), block_registry.clone());

            surface_painter.paint_terrain_chunk(
                painter,
                &biome_map,
                &block_registry,
                &biome_registry,
            );
        })
    });

    group.finish();
}

criterion_group!(benches, bench_chunk_generation);
criterion_main!(benches);

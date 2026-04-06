use bevy::ecs::prelude::World;
use client::prelude::*;
use client::render::textures::VoxelTextureProcessor;
use client::settings::ClientSettings;
use criterion::{Criterion, criterion_group, criterion_main};
use shared::simulation::{
    biome::biome_registry::BiomeRegistryResource,
    block::{BlockRegistryResource, SOLID_BLOCK_ID},
    chunk::{
        ChunkDataOption, NeighborLODs, PaddedChunk, build_chunk_mesh,
        components::{ChunkBlocksComponent, ChunkCoord},
        thread_buffer_pool::acquire_buffer,
        types::ChunkLod,
    },
    terrain::{
        BasicBiomeGenerator, BiomeGenerator, BiomeMapComponent, BiomeResultBuilder,
        ClimateGenerator, ClimateNoiseGenerator, PaintResultBuilder, ShapeResultBuilder,
        SimpleSurfacePainter, SinwaveShaper, TerrainPainter, TerrainShaper,
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

    let persistent_paths = PersistentPaths::resolve_dev();
    let client_settings = ClientSettings::load_or_create(&persistent_paths);

    let mut world = World::new();
    world.insert_resource(
        VoxelTextureProcessor::new(
            persistent_paths.assets_dir.clone(),
            &client_settings.texture_pack,
        )
        .create_registry()
        .unwrap(),
    );
    world.insert_resource(persistent_paths.clone());

    world.init_resource::<BlockRegistryResource>();
    world.init_resource::<BiomeRegistryResource>();

    let block_registry = world.resource::<BlockRegistryResource>().clone();
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

    let sinwave_shaper = SinwaveShaper::new();
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

    let surface_painter = SimpleSurfacePainter::new();
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

fn bench_chunk_meshing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Chunk meshing");

    // INFO: ---------------
    //         setup
    // ---------------------

    let persistent_paths = PersistentPaths::resolve_dev();
    let client_settings = ClientSettings::load_or_create(&persistent_paths);

    let mut world = World::new();
    world.insert_resource(
        VoxelTextureProcessor::new(
            persistent_paths.assets_dir.clone(),
            &client_settings.texture_pack,
        )
        .create_registry()
        .unwrap(),
    );
    world.insert_resource(persistent_paths.clone());
    world.init_resource::<BlockRegistryResource>();

    let block_registry = world.resource::<BlockRegistryResource>().clone();

    // INFO: ---------------------------------
    //         dense meshing benchmark
    // ---------------------------------------

    let mut dense_chunks: [[[ChunkDataOption; 3]; 3]; 3] = Default::default();
    let solid_chunk =
        ChunkDataOption::Generated(ChunkBlocksComponent::new_uniform_solid(ChunkLod(0)));

    #[allow(clippy::needless_range_loop)]
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                if x == 1 && y == 1 && z == 1 {
                    continue;
                }
                dense_chunks[x][y][z] = solid_chunk.clone();
            }
        }
    }

    // default to water for more complex meshing with transparency
    let mut center_chunk = ChunkBlocksComponent::new_uniform(
        ChunkLod(0),
        block_registry.get_block_id_by_name("water").unwrap(),
    );
    let size = center_chunk.size();
    let mut writer = center_chunk.get_writer();
    // create a y=x "slope" chunk
    for x in 0..size {
        for y in 0..=x {
            for z in 0..size {
                writer.set_data(x, y, z, SOLID_BLOCK_ID);
            }
        }
    }
    dense_chunks[1][1][1] = ChunkDataOption::Generated(center_chunk);

    let dense_neighbor_lods = NeighborLODs::default();

    group.bench_function("dense meshing", |b| {
        b.iter(|| {
            let buffer = acquire_buffer();
            let dense_padded_chunk =
                PaddedChunk::new(&dense_chunks, ChunkLod(0), dense_neighbor_lods, buffer);
            build_chunk_mesh("bench_chunk_dense", &dense_padded_chunk, &block_registry)
        })
    });

    // INFO: --------------------------------
    //         hull meshing benchmark
    // --------------------------------------

    let mut hull_chunks: [[[ChunkDataOption; 3]; 3]; 3] = Default::default();
    let empty_chunk =
        ChunkDataOption::Generated(ChunkBlocksComponent::new_uniform_empty(ChunkLod(0)));

    #[allow(clippy::needless_range_loop)]
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                hull_chunks[x][y][z] = empty_chunk.clone();
            }
        }
    }

    hull_chunks[1][1][1] =
        ChunkDataOption::Generated(ChunkBlocksComponent::new_uniform_solid(ChunkLod(0)));

    let hull_neighbor_lods = NeighborLODs::default();

    group.bench_function("hull meshing", |b| {
        b.iter(|| {
            let buffer = acquire_buffer();
            let hull_padded_chunk =
                PaddedChunk::new(&hull_chunks, ChunkLod(0), hull_neighbor_lods, buffer);
            build_chunk_mesh("bench_chunk_hull", &hull_padded_chunk, &block_registry)
        })
    });
}

criterion_group!(benches, bench_chunk_generation, bench_chunk_meshing);
criterion_main!(benches);

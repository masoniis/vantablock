use client::render::chunk::meshing::build_chunk_mesh;
use client::render::{block::BlockRenderDataRegistry, texture::VoxelTextureProcessor};
use client::settings::ClientSettings;
use criterion::{Criterion, criterion_group, criterion_main};
use shared::simulation::{
    block::{BlockRegistry, SOLID_BLOCK_ID},
    chunk::{
        ChunkDataOption, NeighborLODs, PaddedChunk, components::ChunkBlocksComponent,
        thread_buffer_pool::acquire_buffer, types::ChunkLod,
    },
};
use utils::PersistentPaths;

fn bench_chunk_meshing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Chunk meshing");

    // INFO: ---------------
    //         setup
    // ---------------------

    let persistent_paths = PersistentPaths::resolve();
    let client_settings = ClientSettings::load_or_create(&persistent_paths);

    // meshing requires textures to resolve
    let (_texture_array, texture_registry) = VoxelTextureProcessor::new(
        persistent_paths.assets_dir.clone(),
        &client_settings.texture_pack,
    )
    .load_and_stitch()
    .expect("Failed to load and stitch textures");

    let block_registry = BlockRegistry::load_from_disk(&persistent_paths);
    let render_registry = BlockRenderDataRegistry::load_from_disk(
        &persistent_paths,
        &block_registry,
        &texture_registry,
    );

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
    let texture_lut = render_registry.get_texture_lut();

    group.bench_function("dense meshing", |b| {
        b.iter(|| {
            let buffer = acquire_buffer();
            let dense_padded_chunk =
                PaddedChunk::new(&dense_chunks, ChunkLod(0), dense_neighbor_lods, buffer);
            build_chunk_mesh(
                "bench_chunk_dense",
                &dense_padded_chunk,
                &block_registry,
                &render_registry,
                texture_lut,
            )
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
            build_chunk_mesh(
                "bench_chunk_hull",
                &hull_padded_chunk,
                &block_registry,
                &render_registry,
                texture_lut,
            )
        })
    });
}

criterion_group!(benches, bench_chunk_meshing);
criterion_main!(benches);

use crate::prelude::*;
use crate::simulation_world::chunk::{
    chunk_blocks::ChunkView, types::ChunkLod, ChunkBlocksComponent, CHUNK_SIDE_LENGTH,
};

/// Upsamples a low-LOD (e.g., 16x16x16) chunk to a higher LOD (e.g., 32x32x32).
#[instrument(skip_all)]
pub fn upsample_chunk(
    chunk_to_upsample: &ChunkBlocksComponent,
    target_lod: ChunkLod,
) -> ChunkBlocksComponent {
    let target_size = CHUNK_SIDE_LENGTH >> *target_lod;

    if cfg!(debug_assertions) && *target_lod >= *chunk_to_upsample.lod() {
        panic!("upsample_chunk: target_lod must be higher detail than source lod");
    }

    let lod_diff_shift = *chunk_to_upsample.lod() - *target_lod;

    match chunk_to_upsample.get_view() {
        ChunkView::Uniform(block_id) => ChunkBlocksComponent::new_uniform(target_lod, block_id),
        ChunkView::Dense(volume_view) => {
            let mut target_chunk = ChunkBlocksComponent::new_dense_zeroed(target_lod);

            {
                let mut writer = target_chunk.get_writer();

                for x in 0..target_size {
                    for z in 0..target_size {
                        for y in 0..target_size {
                            // map target coord -> source coord
                            let lod_x = x >> lod_diff_shift;
                            let lod_y = y >> lod_diff_shift;
                            let lod_z = z >> lod_diff_shift;

                            // update
                            let block_id = volume_view.get_data(lod_x, lod_y, lod_z);
                            writer.set_data(x, y, z, block_id);
                        }
                    }
                }
            }

            target_chunk
        }
    }
}

/// Downsamples a high-LOD (e.g., 32x32x32) chunk to a lower LOD (e.g., 16x16x16).
#[instrument(skip_all)]
pub fn downsample_chunk(
    chunk_to_downsample: &ChunkBlocksComponent,
    target_lod: ChunkLod,
) -> ChunkBlocksComponent {
    let target_size = CHUNK_SIDE_LENGTH >> *target_lod;

    if cfg!(debug_assertions) && *target_lod <= *chunk_to_downsample.lod() {
        panic!("downsample_chunk: target_lod must be lower detail than source lod");
    }

    let lod_diff_shift = *target_lod - *chunk_to_downsample.lod();

    match chunk_to_downsample.get_view() {
        ChunkView::Uniform(block_id) => ChunkBlocksComponent::new_uniform(target_lod, block_id),
        ChunkView::Dense(volume_view) => {
            let mut target_chunk = ChunkBlocksComponent::new_dense_zeroed(target_lod);

            {
                let mut writer = target_chunk.get_writer();

                for x in 0..target_size {
                    for z in 0..target_size {
                        for y in 0..target_size {
                            // map coord to target (simple donwsample that just chooses first block)
                            let source_x = x << lod_diff_shift;
                            let source_y = y << lod_diff_shift;
                            let source_z = z << lod_diff_shift;

                            // write to target
                            let block_id = volume_view.get_data(source_x, source_y, source_z);
                            writer.set_data(x, y, z, block_id);
                        }
                    }
                }
            }

            target_chunk
        }
    }
}

pub mod common;
pub mod dense;
pub mod hull;
pub mod packed_face;

// INFO: --------------------------------
//         public mesh entrypoint
// --------------------------------------

use crate::prelude::*;
use crate::simulation::{
    asset_management::VoxelChunkMeshAsset,
    block::{
        BlockRegistryResource,
        block_registry::{AIR_BLOCK_ID, BlockId},
    },
    chunk::PaddedChunk,
};

// convenience mesh types
pub type OpaqueMeshData = VoxelChunkMeshAsset;
pub type TransparentMeshData = VoxelChunkMeshAsset;

/// Main chunk meshing entry point: Build a mesh for a single chunk.
#[instrument(skip_all, fields(chunk = %name))]
pub fn build_chunk_mesh(
    name: &str,
    padded_chunk: &PaddedChunk,
    block_registry: &BlockRegistryResource,
) -> (Option<OpaqueMeshData>, Option<TransparentMeshData>) {
    match padded_chunk.get_center_uniform_block() {
        Some(block_id) => {
            if block_id == AIR_BLOCK_ID || is_fully_occluded(padded_chunk, block_registry, block_id)
            {
                return (None, None);
            }

            // only need to hull mesh if chunk is not occluded and is not air
            hull::build_hull_mesh(name, padded_chunk, block_registry, block_id)
        }
        // otherwise do a full dense mesh
        None => dense::build_dense_mesh(name, padded_chunk, block_registry),
    }
}

/// Helper to check if a chunk is completely hidden (surrounded by solid opaque neighbors).
#[instrument(skip_all)]
fn is_fully_occluded(
    padded: &PaddedChunk,
    registry: &BlockRegistryResource,
    center_id: BlockId,
) -> bool {
    let center_props = registry.get_render_data(center_id);

    // if center is transparent can't cull
    if center_props.is_transparent {
        return false;
    }

    // check all 6 neighbors for fully opaque, and if all are then we are hidden.
    padded.is_neighbor_fully_opaque(IVec3::Y, registry)
        && padded.is_neighbor_fully_opaque(IVec3::NEG_Y, registry)
        && padded.is_neighbor_fully_opaque(IVec3::NEG_X, registry)
        && padded.is_neighbor_fully_opaque(IVec3::X, registry)
        && padded.is_neighbor_fully_opaque(IVec3::Z, registry)
        && padded.is_neighbor_fully_opaque(IVec3::NEG_Z, registry)
}

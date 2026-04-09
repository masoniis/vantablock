pub mod common;
pub mod dense;
pub mod hull;
pub mod packed_face;

// INFO: --------------------------------
//         public mesh entrypoint
// --------------------------------------

use crate::prelude::*;
use crate::render::chunk::asset::VoxelMeshAsset;
use shared::simulation::{
    block::{
        BlockRegistry,
        block_registry::{AIR_BLOCK_ID, BlockId},
    },
    chunk::PaddedChunk,
};

// convenience mesh types
pub type OpaqueMeshData = VoxelMeshAsset;
pub type TransparentMeshData = VoxelMeshAsset;

/// Main chunk meshing entry point: Build a mesh for a single chunk.
#[instrument(skip_all, fields(chunk = %name))]
pub fn build_chunk_mesh<R>(
    name: &str,
    padded_chunk: &PaddedChunk,
    block_registry: &BlockRegistry,
    render_registry: &R,
    texture_lut: &[[u32; 6]],
) -> (Option<OpaqueMeshData>, Option<TransparentMeshData>) {
    match padded_chunk.get_center_uniform_block() {
        Some(block_id) => {
            if block_id == AIR_BLOCK_ID || is_fully_occluded(padded_chunk, block_registry, block_id)
            {
                return (None, None);
            }

            // only need to hull mesh if chunk is not occluded and is not air
            hull::build_hull_mesh(
                name,
                padded_chunk,
                block_registry,
                render_registry,
                block_id,
                texture_lut,
            )
        }
        // otherwise do a full dense mesh
        None => dense::build_dense_mesh(
            name,
            padded_chunk,
            block_registry,
            render_registry,
            texture_lut,
        ),
    }
}

/// Helper to check if a chunk is completely hidden (surrounded by solid opaque neighbors).
#[instrument(skip_all)]
fn is_fully_occluded(padded: &PaddedChunk, registry: &BlockRegistry, center_id: BlockId) -> bool {
    let is_transparent = registry.get_transparency_lut()[center_id as usize];

    // if center is transparent can't cull
    if is_transparent {
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

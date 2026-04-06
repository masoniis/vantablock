use super::{OpaqueMeshData, TransparentMeshData, common::*};
use crate::prelude::*;
use crate::simulation::{
    block::{BlockRegistryResource, block_registry::AIR_BLOCK_ID},
    chunk::{CHUNK_SIDE_LENGTH, PaddedChunk},
};

/// Standard mesher for dense, mixed-block chunks.
#[instrument(skip_all)]
pub fn build_dense_mesh(
    name: &str,
    padded_chunk: &PaddedChunk,
    block_registry: &BlockRegistryResource,
) -> (Option<OpaqueMeshData>, Option<TransparentMeshData>) {
    // TODO: using a buffer pool is probably better than this alloc guesswork
    // even though we ultimately need a personal copy of the data at the end
    let mut opaque_faces = Vec::with_capacity(20_000);
    let mut transparent_faces = Vec::with_capacity(5_000);

    let ctx = MesherContext {
        padded_chunk,
        block_registry,
        center_lod: padded_chunk.center_lod(),
        neighbor_lods: padded_chunk.neighbor_lods(),
        chunk_size: padded_chunk.get_size(),
        scale: (CHUNK_SIDE_LENGTH / padded_chunk.get_size()) as f32,
    };

    let transparency_lut = block_registry.get_transparency_lut();
    let texture_lut = block_registry.get_texture_lut();

    let size = ctx.chunk_size;

    for x in 0..size {
        for z in 0..size {
            for y in 0..size {
                let pos = IVec3::new(x as i32, y as i32, z as i32);
                let current_block_id = padded_chunk.get_block(pos.x, pos.y, pos.z);

                if current_block_id == AIR_BLOCK_ID {
                    continue;
                }

                let is_current_transparent = transparency_lut[current_block_id as usize];

                let faces = if is_current_transparent {
                    &mut transparent_faces
                } else {
                    &mut opaque_faces
                };

                // iterate each face checking and generating face verts
                for &face_side in &FaceSide::ALL {
                    let face_i = face_side as usize;
                    let offset = NEIGHBOR_OFFSETS[face_i];
                    let neighbor_pos = pos + offset;

                    let neighbor_id =
                        padded_chunk.get_block(neighbor_pos.x, neighbor_pos.y, neighbor_pos.z);

                    let is_neighbor_transparent = transparency_lut[neighbor_id as usize];

                    if should_render_face(
                        current_block_id,
                        is_current_transparent,
                        neighbor_id,
                        is_neighbor_transparent,
                    ) {
                        let tex_id = texture_lut[current_block_id as usize][face_i];

                        let ao = calculate_ao_levels_for_face(
                            pos,
                            face_side,
                            padded_chunk,
                            transparency_lut,
                        );

                        ctx.push_face(face_side, pos, tex_id, ao, faces);
                    }
                }
            }
        }
    }

    build_mesh_assets(name, opaque_faces, transparent_faces)
}

use super::{OpaqueMeshData, TransparentMeshData, common::*};
use crate::prelude::*;
use shared::world::{
    block::{BlockRegistry, block_registry::BlockId},
    chunk::{CHUNK_SIDE_LENGTH, PaddedChunk},
};

/// Optimized mesher for uniform solid chunks.
///
/// Only iterates the 6 boundary faces, skipping the interior.
#[instrument(skip_all)]
pub fn build_hull_mesh<R>(
    name: &str,
    padded_chunk: &PaddedChunk,
    block_registry: &BlockRegistry,
    render_registry: &R,
    block_id: BlockId,
    texture_lut: &[[u32; 6]],
) -> (Option<OpaqueMeshData>, Option<TransparentMeshData>) {
    let transparency_lut = block_registry.get_transparency_lut();
    let is_trans = transparency_lut[block_id as usize];

    // use the appropriate thread-local buffer
    let buffer_id = if is_trans {
        &TRANSPARENT_FACE_BUFFER
    } else {
        &OPAQUE_FACE_BUFFER
    };

    buffer_id.with(|buffer_ref| {
        let mut faces = buffer_ref.borrow_mut();
        faces.clear();

        let ctx = MesherContext {
            padded_chunk,
            block_registry,
            render_registry,
            center_lod: padded_chunk.center_lod(),
            neighbor_lods: padded_chunk.neighbor_lods(),
            chunk_size: padded_chunk.get_size(),
            scale: CHUNK_SIDE_LENGTH as f32 / padded_chunk.get_size() as f32,
        };

        let size = ctx.chunk_size;
        let tex_id = texture_lut[block_id as usize];

        // a macro to iterate a face plane of the chunk cube
        macro_rules! mesh_plane {
            ($face_idx:expr, $u_range:expr, $v_range:expr, $pos_fn:expr) => {
                let offset = NEIGHBOR_OFFSETS[$face_idx];

                if !ctx
                    .padded_chunk
                    .is_neighbor_fully_opaque(offset, ctx.block_registry)
                {
                    for u in $u_range {
                        for v in $v_range {
                            let pos = $pos_fn(u, v);

                            let neighbor_pos = pos + offset;
                            let neighbor_id = ctx.padded_chunk.get_block(
                                neighbor_pos.x,
                                neighbor_pos.y,
                                neighbor_pos.z,
                            );

                            let is_neighbor_trans = transparency_lut[neighbor_id as usize];

                            if should_render_face(
                                block_id,
                                is_trans,
                                neighbor_id,
                                is_neighbor_trans,
                            ) {
                                let ao = calculate_ao_levels_for_face(
                                    pos,
                                    FaceSide::ALL[$face_idx],
                                    ctx.padded_chunk,
                                    transparency_lut,
                                );

                                ctx.push_face(
                                    FaceSide::ALL[$face_idx],
                                    pos,
                                    tex_id[$face_idx],
                                    ao,
                                    &mut faces,
                                );
                            }
                        }
                    }
                }
            };
        }

        // run macro for each face plane
        #[rustfmt::skip]
        mesh_plane!(0, 0..size, 0..size, |x, z| IVec3::new(x as i32, (size - 1) as i32, z as i32));
        #[rustfmt::skip]
        mesh_plane!(1, 0..size, 0..size, |x, z| IVec3::new(x as i32, 0, z as i32));
        #[rustfmt::skip]
        mesh_plane!(2, 0..size, 0..size, |y, z| IVec3::new(0, y as i32, z as i32));
        #[rustfmt::skip]
        mesh_plane!(3, 0..size, 0..size, |y, z| IVec3::new((size - 1) as i32, y as i32, z as i32));
        #[rustfmt::skip]
        mesh_plane!(4, 0..size, 0..size, |x, y| IVec3::new(x as i32, y as i32, (size - 1) as i32));
        #[rustfmt::skip]
        mesh_plane!(5, 0..size, 0..size, |x, y| IVec3::new(x as i32, y as i32, 0));

        if is_trans {
            build_mesh_assets(name, &[], &faces)
        } else {
            build_mesh_assets(name, &faces, &[])
        }
    })
}

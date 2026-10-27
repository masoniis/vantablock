use super::{OpaqueMeshData, TransparentMeshData, common::*};
use crate::prelude::*;
use common::world::{
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
        mesh_plane!(2, 0..size, 0..size, |y, z| IVec3::new((size - 1) as i32, y as i32, z as i32));
        #[rustfmt::skip]
        mesh_plane!(3, 0..size, 0..size, |y, z| IVec3::new(0, y as i32, z as i32));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::chunk::meshing::{PackedFace, dense::build_dense_mesh};
    use common::world::chunk::{
        ChunkDataOption, NeighborLODs, components::ChunkBlocksComponent, types::ChunkLod,
    };
    use std::collections::HashMap;

    const STONE: BlockId = 1;

    /// A registry where only the transparency lut matters (0 = air, 1 = stone).
    fn registry() -> BlockRegistry {
        BlockRegistry::from_raw(vec![true, false], Vec::new(), HashMap::new())
    }

    /// Builds a padded chunk of uniform stone whose neighbors are stone except for `air_neighbors`.
    fn padded_with_air_neighbors(air_neighbors: &[IVec3]) -> PaddedChunk {
        let stone =
            ChunkDataOption::Generated(ChunkBlocksComponent::new_uniform(ChunkLod(0), STONE));
        let air = ChunkDataOption::Generated(ChunkBlocksComponent::new_uniform_empty(ChunkLod(0)));

        let mut chunks: [[[ChunkDataOption; 3]; 3]; 3] = Default::default();
        for (x, plane) in chunks.iter_mut().enumerate() {
            for (y, row) in plane.iter_mut().enumerate() {
                for (z, cell) in row.iter_mut().enumerate() {
                    let offset = IVec3::new(x as i32 - 1, y as i32 - 1, z as i32 - 1);
                    *cell = if air_neighbors.contains(&offset) {
                        air.clone()
                    } else {
                        stone.clone()
                    };
                }
            }
        }

        PaddedChunk::new(&chunks, ChunkLod(0), NeighborLODs::default(), Vec::new())
    }

    fn sorted_faces(
        mesh: (
            Option<crate::render::chunk::meshing::OpaqueMeshData>,
            Option<crate::render::chunk::meshing::TransparentMeshData>,
        ),
    ) -> Vec<PackedFace> {
        let mut faces = mesh.0.map(|m| m.faces.to_vec()).unwrap_or_default();
        faces.sort();
        faces
    }

    /// The hull mesher is an optimization of the dense mesher, so for a uniform chunk both
    /// must emit exactly the same faces regardless of which neighbors are open.
    #[test]
    fn hull_matches_dense_for_every_open_side() {
        let registry = registry();
        let texture_lut = vec![[0u32; 6]; 2];

        for open in common::world::chunk::NEIGHBOR_OFFSETS {
            let padded = padded_with_air_neighbors(&[open]);

            let hull = sorted_faces(build_hull_mesh(
                "hull",
                &padded,
                &registry,
                &(),
                STONE,
                &texture_lut,
            ));
            let dense = sorted_faces(build_dense_mesh(
                "dense",
                &padded,
                &registry,
                &(),
                &texture_lut,
            ));

            assert_eq!(
                hull, dense,
                "hull and dense disagree with open neighbor {open}"
            );
        }
    }
}

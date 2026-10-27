use crate::{
    render::{
        block::BlockRenderDataRegistry,
        chunk::meshing::{OpaqueMeshData, TransparentMeshData, build_chunk_mesh},
    },
    world::chunk::{ClientChunkMap, range::is_in_bounds},
};
use bevy::{ecs::prelude::*, math::IVec3, tasks::AsyncComputeTaskPool};
use common::world::{
    block::BlockRegistry,
    chunk::{
        ChunkBlocksComponent, ChunkCoord, ChunkLod, NEIGHBOR_OFFSETS,
        common::{
            chunk_scaling::{downsample_chunk, upsample_chunk},
            padded_chunk_view::{ChunkDataOption, NeighborLODs, PaddedChunk},
            thread_buffer_pool::{acquire_buffer, release_buffer},
        },
    },
};
use crossbeam::channel::{Receiver, unbounded};

/// A meshing task in flight for a chunk entity. Its result is collected by `poll`.
#[derive(Component)]
pub struct ChunkMeshingTaskComponent {
    pub receiver: Receiver<(Option<OpaqueMeshData>, Option<TransparentMeshData>)>,
}

/// Everything a meshing task needs: the chunk and its 26 neighbors, resampled to its LOD.
pub struct MeshingInput {
    coord: ChunkCoord,
    chunks: [[[ChunkDataOption; 3]; 3]; 3],
    center_lod: ChunkLod,
    neighbor_lods: NeighborLODs,
}

impl MeshingInput {
    /// Collects the chunk's data and its neighbors', or `None` while any in-bounds neighbor
    /// has no data yet (faces on the shared border depend on it).
    pub fn gather(
        coord: IVec3,
        center: &ChunkBlocksComponent,
        chunk_map: &ClientChunkMap,
        blocks: &Query<&ChunkBlocksComponent>,
    ) -> Option<Self> {
        let center_lod = center.lod();
        let mut chunks: [[[ChunkDataOption; 3]; 3]; 3] = Default::default();
        let mut neighbor_lods = NeighborLODs::default();
        chunks[1][1][1] = ChunkDataOption::Generated(center.clone());

        for offset in NEIGHBOR_OFFSETS {
            let neighbor_coord = coord + offset;
            let (data, lod) = if is_in_bounds(neighbor_coord) {
                let entity = chunk_map.get_entity(neighbor_coord)?;
                let neighbor = blocks.get(entity).ok()?;
                let lod = neighbor.lod();
                let resampled = if lod > center_lod {
                    upsample_chunk(neighbor, center_lod)
                } else if lod < center_lod {
                    downsample_chunk(neighbor, center_lod)
                } else {
                    neighbor.clone()
                };
                (ChunkDataOption::Generated(resampled), Some(lod))
            } else {
                (ChunkDataOption::OutOfBounds, None)
            };

            // map the offset (-1, 0, 1) to an array index (0, 1, 2)
            let [x, y, z] = (offset + IVec3::ONE).to_array().map(|i| i as usize);
            chunks[x][y][z] = data;
            neighbor_lods[x][y][z] = lod;
        }

        Some(Self {
            coord: ChunkCoord { pos: coord },
            chunks,
            center_lod,
            neighbor_lods,
        })
    }

    /// Meshes the chunk on the async compute pool.
    pub fn spawn(
        self,
        block_registry: &BlockRegistry,
        render_registry: &BlockRenderDataRegistry,
    ) -> ChunkMeshingTaskComponent {
        let block_registry = block_registry.clone();
        let render_registry = render_registry.clone();
        let (sender, receiver) = unbounded();

        AsyncComputeTaskPool::get()
            .spawn(async move {
                let padded = PaddedChunk::new(
                    &self.chunks,
                    self.center_lod,
                    self.neighbor_lods,
                    acquire_buffer(),
                );
                let meshes = build_chunk_mesh(
                    &self.coord.to_string(),
                    &padded,
                    &block_registry,
                    &render_registry,
                    render_registry.get_texture_lut(),
                );
                release_buffer(padded.take_buffer());

                // the receiver is gone if the chunk was unloaded meanwhile
                let _ = sender.send(meshes);
            })
            .detach();

        ChunkMeshingTaskComponent { receiver }
    }
}

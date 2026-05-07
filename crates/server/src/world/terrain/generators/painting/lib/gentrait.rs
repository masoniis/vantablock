use crate::prelude::*;
use crate::world::terrain::BiomeMapComponent;
use shared::world::{
    biome::BiomeRegistryResource,
    block::{BlockId, BlockRegistry},
    chunk::{ChunkBlocksComponent, ChunkCoord, ChunkMetadata, VolumeDataWriter},
};
use std::fmt::Debug;

// INFO: -------------------------
//         terrain painter
// -------------------------------

pub trait TerrainPainter: Send + Sync + Debug {
    fn paint_terrain_chunk(
        &self,
        painter: PaintResultBuilder,

        biome_map: &BiomeMapComponent,

        block_registry: &BlockRegistry,
        biome_registry: &BiomeRegistryResource,
    ) -> PaintResultBuilder;
}

pub struct PaintResultBuilder {
    blocks: ChunkBlocksComponent,
    pub chunk_coord: ChunkCoord,
    metadata: ChunkMetadata,
    block_registry: BlockRegistry,
}

impl PaintResultBuilder {
    /// Creates a new painter, taking ownership of the ChunkBlocksComponent.
    pub fn new(
        blocks: ChunkBlocksComponent,
        chunk_coord: ChunkCoord,
        block_registry: BlockRegistry,
    ) -> Self {
        Self {
            blocks,
            chunk_coord,
            metadata: ChunkMetadata::new(),
            block_registry,
        }
    }

    /// Returns the size of the chunk.
    pub fn size(&self) -> usize {
        self.blocks.size()
    }

    /// Returns a read-only view (useful for early exit checks).
    pub fn is_uniform(&self) -> Option<BlockId> {
        self.blocks.is_uniform()
    }

    /// Opens a high-performance edit scope.
    #[inline(always)]
    pub fn edit_arbitrary(&mut self, mut f: impl FnMut(&mut PaintWriter)) {
        let block_writer = self.blocks.get_writer();

        let mut writer = PaintWriter {
            block_writer,
            metadata: &mut self.metadata,
            registry: &self.block_registry,
        };

        f(&mut writer);
    }

    /// Runs an optimally structured loop (X, Z, Y) for painting logic.
    ///
    /// The closure receives:
    /// 1. Local Coordinate (x, y, z)
    /// 2. World Coordinate (wx, wy, wz)
    /// 3. Current Block ID
    ///
    /// It should return `Some(NewBlockId)` to change the block, or `None` to leave it alone.
    #[inline(always)]
    pub fn fill_from(&mut self, f: impl Fn(IVec3, IVec3, BlockId) -> Option<BlockId>) {
        let size = self.blocks.size() as i32;
        let lod = self.blocks.lod();

        let base_world = self.chunk_coord.as_world_pos();
        let step = 1 << lod.0;

        self.edit_arbitrary(|writer| {
            let base_x = base_world.x;
            let base_y = base_world.y;
            let base_z = base_world.z;

            for x in 0..size {
                let world_x = base_x + (x * step);
                for z in 0..size {
                    let world_z = base_z + (z * step);
                    for y in 0..size {
                        let world_y = base_y + (y * step);

                        let local = IVec3::new(x, y, z);
                        let world = IVec3::new(world_x, world_y, world_z);

                        // call closure with args, updating if relevant
                        let current_block = writer.get_block(x as usize, y as usize, z as usize);
                        if let Some(new_block) = f(local, world, current_block) {
                            writer.set_block(x as usize, y as usize, z as usize, new_block);
                        }
                    }
                }
            }
        });
    }

    /// Consumes the builder and returns the final generated components.
    pub fn finish(self) -> (ChunkBlocksComponent, ChunkMetadata) {
        (self.blocks, self.metadata)
    }
}

/// A temporary helper for writing blocks and updating metadata efficiently.
pub struct PaintWriter<'a> {
    block_writer: VolumeDataWriter<'a, BlockId>,
    metadata: &'a mut ChunkMetadata,
    registry: &'a BlockRegistry,
}

impl<'a> PaintWriter<'a> {
    /// Sets a block in the chunk and updates metadata (uniformity, transparency).
    #[inline(always)]
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_id: BlockId) {
        self.block_writer.set_data(x, y, z, block_id);
        self.update_metadata(block_id);
    }

    /// Fills the entire chunk with a single block efficiently.
    #[inline(always)]
    pub fn fill(&mut self, block_id: BlockId) {
        self.block_writer.fill(block_id)
    }

    /// Gets a block from the chunk.
    #[inline(always)]
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockId {
        self.block_writer.get_data(x, y, z)
    }

    #[inline(always)]
    fn update_metadata(&mut self, block_id: BlockId) {
        // uniformity
        if self.metadata.is_uniform {
            if let Some(first) = self.metadata.uniform_block_id {
                if first != block_id {
                    self.metadata.is_uniform = false;
                    self.metadata.uniform_block_id = None;
                }
            } else {
                self.metadata.uniform_block_id = Some(block_id);
            }
        }

        // transparency
        if !self.metadata.contains_transparent {
            let is_transparent = self.registry.get_transparency_lut()[block_id as usize];
            if is_transparent {
                self.metadata.contains_transparent = true;
            }
        }
    }
}

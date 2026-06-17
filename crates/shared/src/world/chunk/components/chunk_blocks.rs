use crate::world::block::{AIR_BLOCK_ID, BlockId, SOLID_BLOCK_ID};
use crate::world::chunk::{
    CHUNK_SIDE_LENGTH, ChunkLod, ChunkVolumeData, VolumeDataView, VolumeDataWriter,
};
use bevy::ecs::prelude::Component;

/// A read-only view into the chunk's data.
#[derive(Clone, Copy)]
pub enum ChunkView<'a> {
    /// The chunk is uniform (all blocks within it are the same type).
    ///
    /// Contains the single BlockId.
    Uniform(BlockId),

    /// The chunk is dense (not all blocks within it are the same type).
    ///
    /// Contains a high-speed reader struct for hot loops.
    Dense(VolumeDataView<'a, BlockId>),
}

/// A container holding the data blocks within a chunk.
#[derive(Clone)]
pub enum ChunkData {
    Uniform(BlockId),
    Dense(ChunkVolumeData<BlockId>),
}

#[derive(Component, Clone)]
pub struct ChunkBlocksComponent {
    data: ChunkData,
    lod: ChunkLod,
}

impl ChunkBlocksComponent {
    // INFO: ----------------------
    //         constructors
    // ----------------------------

    /// Creates a new chunk filled with a single block type.
    ///
    /// WARN: Calling the unsafe block setter on a uniform chunk WILL result
    /// in crashing the program.
    pub fn new_uniform(lod: ChunkLod, block_id: BlockId) -> Self {
        Self {
            data: ChunkData::Uniform(block_id),
            lod,
        }
    }

    /// Creates a new chunk filled with AIR.
    pub fn new_uniform_empty(lod: ChunkLod) -> Self {
        ChunkBlocksComponent::new_uniform(lod, AIR_BLOCK_ID)
    }

    /// Creates a new chunk filled with the default `SOLID_BLOCK`.
    pub fn new_uniform_solid(lod: ChunkLod) -> Self {
        ChunkBlocksComponent::new_uniform(lod, SOLID_BLOCK_ID)
    }

    /// Creates a new, dense chunk initialized to AIR (0).
    ///
    /// **Performance Note:** This is the **fastest** way to create a new chunk
    /// for generation. It performs a single allocation and a fast memset.
    pub fn new_dense_zeroed(lod: ChunkLod) -> Self {
        Self {
            data: ChunkData::Dense(ChunkVolumeData::new_zeroed(lod)),
            lod,
        }
    }

    /// Creates a new, dense chunk filled with a specific block.
    ///
    /// **Performance Note:** Faster than `from_vec`, but slightly slower than `zeroed`.
    pub fn new_dense_filled(lod: ChunkLod, block_id: BlockId) -> Self {
        Self {
            data: ChunkData::Dense(ChunkVolumeData::new_filled(lod, block_id)),
            lod,
        }
    }

    /// Creates a new dense chunk from a vector.
    ///
    /// Should be used as a last resort as it is slower than other methods, but I
    /// think it will be useful for scenarios like loading from disk, for example.
    pub fn from_vec(lod: ChunkLod, block_data: Vec<BlockId>) -> Self {
        Self {
            data: ChunkData::Dense(ChunkVolumeData::from_vec(lod, block_data)),
            lod,
        }
    }

    // INFO: ---------------
    //         utils
    // ---------------------

    /// Checks if the chunk is Uniform.
    ///
    /// Returns `Some(block_id)` if uniform, `None` if dense.
    pub fn is_uniform(&self) -> Option<BlockId> {
        match &self.data {
            ChunkData::Uniform(block_id) => Some(*block_id),
            ChunkData::Dense(_) => None,
        }
    }

    // INFO: -----------------
    //         getters
    // -----------------------

    /// Returns the Level of Detail (LOD) of this chunk.
    pub fn lod(&self) -> ChunkLod {
        self.lod
    }

    /// Returns the side length of this chunk (e.g., 32).
    pub fn size(&self) -> usize {
        CHUNK_SIDE_LENGTH >> *self.lod()
    }

    /// Gets a read-only view of the chunk data.
    pub fn get_view(&self) -> ChunkView<'_> {
        match &self.data {
            ChunkData::Uniform(block_id) => ChunkView::Uniform(*block_id),
            ChunkData::Dense(volume) => ChunkView::Dense(volume.get_data_view()),
        }
    }

    /// Prepares the chunk for batch writing and returns a high-speed accessor.
    ///
    /// 1. Converts a `Uniform` chunk to `Dense` if the chunk is not already `Dense`.
    /// 2. Create a mutable copy of the chunk if multiple threads are using it.
    /// 3. Return an accessor struct used to edit the data in an optimized fashion.
    ///
    /// WARN: Calling this on a chunk unnecessarily will convert it to the the
    /// less efficient `Dense` format. Only call this in a situation where the
    /// chunk is guaranteed to be altered to a `Dense` (non-uniform) state.
    #[inline(always)]
    pub fn get_writer(&mut self) -> VolumeDataWriter<'_, BlockId> {
        // ensure chunk is dense, or convert if not
        if let ChunkData::Uniform(uniform_block_id) = self.data {
            let dense_volume = ChunkVolumeData::new_filled(self.lod(), uniform_block_id);
            self.data = ChunkData::Dense(dense_volume);
        }

        // get the data accessor
        match &mut self.data {
            ChunkData::Dense(volume) => volume.get_data_writer(),
            ChunkData::Uniform(_) => unreachable!(),
        }
    }
}

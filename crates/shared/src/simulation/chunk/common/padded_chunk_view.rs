use super::TOTAL_BUFFER_SIZE;
use crate::prelude::*;
use crate::simulation::{
    block::block_registry::{AIR_BLOCK_ID, BlockId, BlockRegistry},
    chunk::{CHUNK_SIDE_LENGTH, ChunkBlocksComponent, ChunkLod, ChunkView},
};

pub const PADDED_SIZE: usize = CHUNK_SIDE_LENGTH + 2;

/// Holds the *original* LODs of all 26 neighbors (and the center)
/// in a 3x3x3 grid. [1][1][1] is the center.
pub type NeighborLODs = [[[Option<ChunkLod>; 3]; 3]; 3];

/// Represents the state of neighbor chunk data passed to the mesher.
#[derive(Clone, Default)]
pub enum ChunkDataOption {
    /// The chunk's block data is available.
    Generated(ChunkBlocksComponent),
    /// The chunk coordinate is outside the world's bounds.
    OutOfBounds,
    /// The chunk is within bounds but has no data (e.g., not generated).
    #[default]
    Empty,
}

/// A flat, cache-optimized buffer containing the chunk + 1 layer of neighbors.
#[derive(Clone)]
pub struct PaddedChunk {
    /// Padded chunk volume with side-length of size `PADDED_SIZE`.
    voxels: Vec<BlockId>,

    /// The LOD of the center chunk
    center_lod: ChunkLod,
    /// The LOD of all 26 chunk neighbors
    neighbor_lods: NeighborLODs,

    /// Stores the block ID of the center chunk if uniform, or None if dense.
    center_uniform_block: Option<BlockId>,
}

impl PaddedChunk {
    /// Creates a PaddedChunk using a recycled buffer passed in by the caller.
    pub fn new(
        chunks: &[[[ChunkDataOption; 3]; 3]; 3],
        center_lod: ChunkLod,
        neighbor_lods: NeighborLODs,
        mut buffer: Vec<BlockId>,
    ) -> Self {
        // save center uniform from input
        let center_uniform_block = match &chunks[1][1][1] {
            ChunkDataOption::Generated(comp) => match comp.get_view() {
                ChunkView::Uniform(id) => Some(id),
                ChunkView::Dense(_) => None,
            },
            _ => Some(AIR_BLOCK_ID),
        };

        // clear stale buffer
        buffer.clear();
        buffer.resize(TOTAL_BUFFER_SIZE, AIR_BLOCK_ID);

        // util for filling the padded chunk
        {
            let mut write_chunk_to_buffer = |offset: IVec3, view: ChunkView| {
                let x_range = match offset.x {
                    -1 => 0..1,
                    0 => 1..CHUNK_SIDE_LENGTH + 1,
                    1 => CHUNK_SIDE_LENGTH + 1..CHUNK_SIDE_LENGTH + 2,
                    _ => 0..0,
                };
                let y_range = match offset.y {
                    -1 => 0..1,
                    0 => 1..CHUNK_SIDE_LENGTH + 1,
                    1 => CHUNK_SIDE_LENGTH + 1..CHUNK_SIDE_LENGTH + 2,
                    _ => 0..0,
                };
                let z_range = match offset.z {
                    -1 => 0..1,
                    0 => 1..CHUNK_SIDE_LENGTH + 1,
                    1 => CHUNK_SIDE_LENGTH + 1..CHUNK_SIDE_LENGTH + 2,
                    _ => 0..0,
                };

                match view {
                    ChunkView::Uniform(block) => {
                        for x in x_range.clone() {
                            for z in z_range.clone() {
                                for y in y_range.clone() {
                                    let idx = y + z * PADDED_SIZE + x * PADDED_SIZE * PADDED_SIZE;
                                    unsafe {
                                        *buffer.get_unchecked_mut(idx) = block;
                                    }
                                }
                            }
                        }
                    }
                    ChunkView::Dense(data) => {
                        for x in x_range {
                            for z in z_range.clone() {
                                for y in y_range.clone() {
                                    // map padded coords back to source chunk coords
                                    let src_x = if offset.x == -1 {
                                        31
                                    } else if offset.x == 1 {
                                        0
                                    } else {
                                        x - 1
                                    };
                                    let src_y = if offset.y == -1 {
                                        31
                                    } else if offset.y == 1 {
                                        0
                                    } else {
                                        y - 1
                                    };
                                    let src_z = if offset.z == -1 {
                                        31
                                    } else if offset.z == 1 {
                                        0
                                    } else {
                                        z - 1
                                    };

                                    let block = data.get_data(src_x, src_y, src_z);
                                    let idx = y + z * PADDED_SIZE + x * PADDED_SIZE * PADDED_SIZE;
                                    unsafe {
                                        *buffer.get_unchecked_mut(idx) = block;
                                    }
                                }
                            }
                        }
                    }
                }
            };

            // iterate over chunk grid and fill
            for (cx, row) in chunks.iter().enumerate().take(3) {
                for (cy, column) in row.iter().enumerate().take(3) {
                    for (cz, cell) in column.iter().enumerate().take(3) {
                        let offset = IVec3::new(cx as i32 - 1, cy as i32 - 1, cz as i32 - 1);

                        match cell {
                            ChunkDataOption::Generated(comp) => {
                                if comp.lod() != center_lod {
                                    // Handle LOD mismatch / resampling here if needed
                                    write_chunk_to_buffer(offset, comp.get_view());
                                } else {
                                    write_chunk_to_buffer(offset, comp.get_view());
                                }
                            }
                            ChunkDataOption::OutOfBounds => {
                                // out of bounds just leave it be
                            }
                            _ => {} // Leave as Air
                        }
                    }
                }
            }
        }

        Self {
            voxels: buffer,
            center_lod,
            neighbor_lods,
            center_uniform_block,
        }
    }

    /// Consumes the PaddedChunk and returns the underlying buffer.
    pub fn take_buffer(self) -> Vec<BlockId> {
        self.voxels
    }

    /// Hot loop accessor.
    #[inline(always)]
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> BlockId {
        let px = (x + 1) as usize;
        let py = (y + 1) as usize;
        let pz = (z + 1) as usize;
        let idx = py + pz * PADDED_SIZE + px * PADDED_SIZE * PADDED_SIZE;
        unsafe { *self.voxels.get_unchecked(idx) }
    }

    /// Gets the center uniform block option.
    #[inline(always)]
    pub fn get_center_uniform_block(&self) -> Option<BlockId> {
        self.center_uniform_block
    }

    /// The LOD of the center chunk.
    pub fn center_lod(&self) -> ChunkLod {
        self.center_lod
    }

    /// The neighbor lod list.
    pub fn neighbor_lods(&self) -> &NeighborLODs {
        &self.neighbor_lods
    }

    /// The size of the padded chunk
    pub fn get_size(&self) -> usize {
        CHUNK_SIDE_LENGTH >> *self.center_lod()
    }

    /// Returns whether or not a particular neighbor offset from the center chunk is fully opaque.
    pub fn is_neighbor_fully_opaque(&self, offset: IVec3, registry: &BlockRegistry) -> bool {
        let (x_range, y_range, z_range) = match (offset.x, offset.y, offset.z) {
            (1, 0, 0) => (
                (CHUNK_SIDE_LENGTH + 1)..(CHUNK_SIDE_LENGTH + 2),
                1..(CHUNK_SIDE_LENGTH + 1),
                1..(CHUNK_SIDE_LENGTH + 1),
            ),
            (-1, 0, 0) => (0..1, 1..(CHUNK_SIDE_LENGTH + 1), 1..(CHUNK_SIDE_LENGTH + 1)),
            (0, 1, 0) => (
                1..(CHUNK_SIDE_LENGTH + 1),
                (CHUNK_SIDE_LENGTH + 1)..(CHUNK_SIDE_LENGTH + 2),
                1..(CHUNK_SIDE_LENGTH + 1),
            ),
            (0, -1, 0) => (1..(CHUNK_SIDE_LENGTH + 1), 0..1, 1..(CHUNK_SIDE_LENGTH + 1)),
            (0, 0, 1) => (
                1..(CHUNK_SIDE_LENGTH + 1),
                1..(CHUNK_SIDE_LENGTH + 1),
                (CHUNK_SIDE_LENGTH + 1)..(CHUNK_SIDE_LENGTH + 2),
            ),
            (0, 0, -1) => (1..(CHUNK_SIDE_LENGTH + 1), 1..(CHUNK_SIDE_LENGTH + 1), 0..1),
            _ => return false,
        };

        let transparency_lut = registry.get_transparency_lut();

        for x in x_range {
            for z in z_range.clone() {
                for y in y_range.clone() {
                    let idx = y + z * PADDED_SIZE + x * PADDED_SIZE * PADDED_SIZE;
                    let block_id = self.voxels[idx];
                    if transparency_lut[block_id as usize] {
                        return false;
                    }
                }
            }
        }

        true
    }
}

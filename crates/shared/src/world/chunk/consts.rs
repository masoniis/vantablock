use bevy::math::IVec3;

// INFO: ----------------------
//         chunk sizing
// ----------------------------

/// The number of bitshifts to apply to the chunk_width and height.
///
/// 2^CHUNK_DIM_SHIFT will be the size of each side of the chunks
///
/// Due to vertex pulling, 32 (shift of 4) should be he only choice that will work here
const CHUNK_DIM_SHIFT: usize = 4;

/// The block length of the side of a chunk.
pub const CHUNK_SIDE_LENGTH: usize = 2 << CHUNK_DIM_SHIFT;
pub const CHUNK_WIDTH: usize = CHUNK_SIDE_LENGTH;
pub const CHUNK_HEIGHT: usize = CHUNK_SIDE_LENGTH;
pub const CHUNK_DEPTH: usize = CHUNK_SIDE_LENGTH;

// INFO: -------------------------
//         render distance
// -------------------------------

/// The distance in the xz plane, in chunks, to load around the camera.
pub const RENDER_DISTANCE: i32 = 8;
pub const LOAD_DISTANCE: i32 = RENDER_DISTANCE + 1; // load an extra ring of chunks for efficient meshing

/// The size of the vertical column that we render (chunks above/below will never be generated)
pub const WORLD_MIN_Y_CHUNK: i32 = 0;
pub const WORLD_MAX_Y_CHUNK: i32 = 256 >> (CHUNK_DIM_SHIFT + 1);

// INFO: ------------------------------
//         other derived consts
// ------------------------------------

// Z_SHIFT is log2(CHUNK_WIDTH)
pub const Z_SHIFT: usize = CHUNK_WIDTH.trailing_zeros() as usize;
// Y_SHIFT is log2(CHUNK_WIDTH * CHUNK_DEPTH)
pub const Y_SHIFT: usize = (CHUNK_WIDTH * CHUNK_DEPTH).trailing_zeros() as usize;

// Helper consts for indexing
pub const CHUNK_SIZE: usize = CHUNK_WIDTH * CHUNK_HEIGHT * CHUNK_DEPTH;
pub const CHUNK_AREA: usize = CHUNK_WIDTH * CHUNK_DEPTH;
pub const INDEX_MASK: usize = CHUNK_SIZE - 1;

/// Offsets to find the 26 direct neighbors of a chunk.
pub const NEIGHBOR_OFFSETS: [IVec3; 26] = {
    let mut offsets = [IVec3::ZERO; 26];

    let mut index = 0;
    let mut x = -1;
    while x <= 1 {
        let mut y = -1;
        while y <= 1 {
            let mut z = -1;
            while z <= 1 {
                if x != 0 || y != 0 || z != 0 {
                    offsets[index] = IVec3::new(x, y, z);
                    index += 1;
                }
                z += 1;
            }
            y += 1;
        }
        x += 1;
    }

    offsets
};

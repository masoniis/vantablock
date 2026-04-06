// INFO: ----------------------
//         chunk sizing
// ----------------------------

/// The number of bitshifts to apply to the chunk_width and height
///
/// 2^CHUNK_DIM_SHIFT will be the size of each side of the chunks
///
/// Due to vertex pulling, 32 (shift of 4) should be he only choice that will work here
const CHUNK_DIM_SHIFT: usize = 4;

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

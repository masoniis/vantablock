//! The chunk meshing pipeline's systems, in the order a chunk passes through them:
//!
//! 1. `decompression`: server chunk bytes are decompressed off-thread into `ChunkBlocksComponent`s.
//! 2. `promote`: new chunks in range (and in-range chunks that were waiting) become `WantsMeshing`.
//! 3. `dispatch`: once all neighbors have data, a meshing task is spawned on the compute pool.
//! 4. `poll`: finished tasks are turned into mesh components for the render world.
//!
//! `dirty` feeds edited chunks back into step 3. Marker components live in `components`.

pub mod components;
pub mod decompression;
pub mod dirty;
pub mod dispatch;
pub mod poll;
pub mod promote;

pub use components::{CheckForMeshing, ChunkMeshingTaskComponent, DecompressionTask, WantsMeshing};
pub use decompression::{
    apply_decompressed_chunk_data_system, decompress_chunk_data_system,
    poll_decompression_tasks_system,
};
pub use dirty::handle_dirty_chunks_system;
pub use dispatch::start_pending_meshing_tasks_system;
pub use poll::poll_chunk_meshing_tasks;
pub use promote::{
    manage_distance_based_chunk_meshing_targets_system, promote_newly_generated_chunks_system,
};

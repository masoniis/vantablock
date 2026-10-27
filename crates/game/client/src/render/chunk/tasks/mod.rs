//! The chunk meshing pipeline's systems, in the order a chunk passes through them. Chunks enter
//! once `world::chunk` has received and decompressed their block data.
//!
//! 1. `promote`: new chunks in range (and in-range chunks that were waiting) become `WantsMeshing`.
//! 2. `dispatch`: once all neighbors have data, a meshing task is spawned on the compute pool.
//! 3. `poll`: finished tasks are turned into mesh components for the render world.
//!
//! `dirty` feeds edited chunks back into step 2. Marker components live in `components`.

pub mod components;
pub mod dirty;
pub mod dispatch;
pub mod poll;
pub mod promote;

pub use components::{CheckForMeshing, ChunkMeshingTaskComponent, WantsMeshing};
pub use dirty::handle_dirty_chunks_system;
pub use dispatch::start_pending_meshing_tasks_system;
pub use poll::poll_chunk_meshing_tasks;
pub use promote::{
    manage_distance_based_chunk_meshing_targets_system, promote_newly_generated_chunks_system,
};

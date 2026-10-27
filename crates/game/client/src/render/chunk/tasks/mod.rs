//! Chunk meshing, run once per frame in this order:
//!
//! 1. `dirty`: block edits queue their chunks for a remesh.
//! 2. `poll`: finished tasks become mesh components for the render world.
//! 3. `schedule`: chunks entering render distance are queued, chunks leaving it are dropped
//!    from the queue, and free task slots go to the nearest queued chunks whose neighbors all
//!    have data. `task` builds and spawns that work.
//!
//! The scheduler re-decides everything from `ClientChunkManager` every frame instead of
//! reacting to "a neighbor arrived" signals, so a chunk can't be stranded by a missed signal
//! or a stale marker: if it is queued and in range, it is meshed as soon as it can be.

pub mod dirty;
pub mod poll;
pub mod schedule;
pub mod task;

pub use dirty::handle_dirty_chunks_system;
pub use poll::poll_chunk_meshing_tasks;
pub use schedule::schedule_chunk_meshing_system;
pub use task::ChunkMeshingTaskComponent;

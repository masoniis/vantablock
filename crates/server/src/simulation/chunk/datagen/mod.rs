#[path = "components.rs"]
pub mod gentask_components;
pub mod poll_generation;
pub mod start_generation;

pub use gentask_components::{ChunkGenerationTaskComponent, NeedsGenerating};
pub use poll_generation::poll_chunk_generation_tasks;
pub use start_generation::start_pending_generation_tasks_system;

//! # Async Loading DAG
//!
//! A unified, enum-based Directed Acyclic Graph (DAG) system for managing complex,
//! multi-stage loading sequences in Bevy.
//!
//! This system allows you to define loading "phases" using simple enums, where each variant
//! represents a unique task (node) in the graph. Dependencies between tasks are explicitly
//! defined, and the system automatically orchestrates the execution, ensuring tasks only
//! start when their requirements are met.
//!
//! ## Core Concepts
//!
//! - **DagNodes (Enums):** Your own enum types (e.g., `StartupPhase`) define the nodes.
//!   Each variant is spawned as a dedicated ECS entity when registered.
//! - **Dependencies (Edges):** Explicitly map which nodes must finish before others can start.
//! - **Targeted Observers:** Loading tasks are implemented as Bevy Observers listening for
//!   `StartNode<N>`. The DAG triggers these observers on the specific entity associated
//!   with that enum variant.
//! - **Task Lifecycle:** Nodes move from `Pending` -> `Started` -> `Completed`.
//!
//! ## Manual Polling
//!
//! **Important:** The system does not automatically poll tasks. You must manually add the
//! `poll_tasks::<N>` system to the appropriate schedule (e.g., `Update`) during your loading
//! states to ensure that `LoadingTaskComponent` results are processed and nodes are marked as finished.
//!
//! ## Example
//!
//! ```rust
//! #[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
//! enum MyLoadingPhase {
//!     Assets,
//!     Registry,
//!     Finalize,
//! }
//!
//! impl LoadingDagPhase for MyLoadingPhase {
//!     const PHASE_NAME: &'static str = "MyLoadingPhase";
//! }
//!
//! fn setup(mut app: &mut App) {
//!     app.configure_loading_phase::<MyLoadingPhase>()
//!         .add_node(MyLoadingPhase::Assets, handle_assets)
//!         .add_node(MyLoadingPhase::Registry, handle_registry)
//!         .add_node(MyLoadingPhase::Finalize, handle_finalize)
//!         // Registry depends on Assets
//!         .add_edge(MyLoadingPhase::Assets, MyLoadingPhase::Registry)
//!         // Finalize depends on both Assets and Registry
//!         .add_edge(MyLoadingPhase::Assets, MyLoadingPhase::Finalize)
//!         .add_edge(MyLoadingPhase::Registry, MyLoadingPhase::Finalize);
//!
//!     // Kickoff the loading phase
//!     app.add_systems(OnEnter(MyState::Loading), kickoff_loading_phase::<MyLoadingPhase>);
//!
//!     // IMPORTANT: You must manually register the polling system if your loading tasks are asynchronous
//!     app.add_systems(Update, poll_tasks::<MyLoadingPhase>.run_if(in_state(MyState::Loading)));
//! }
//! ```

mod components;
mod resource;
mod systems;

pub use components::*;
pub use resource::*;
pub use systems::*;

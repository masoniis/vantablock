//! # Async Loading DAG
//!
//! A unified, type-safe Directed Acyclic Graph (DAG) system for managing complex,
//! multi-stage loading sequences in Bevy.
//!
//! This system allows you to define loading "phases" using marker structs, where each
//! task (node) is also represented by a unique component type. Dependencies between
//! tasks are explicitly defined using these types, and the system automatically
//! orchestrates the execution, ensuring tasks only start when their requirements are met.
//!
//! ## Core Concepts
//!
//! - **Phase Markers:** Simple structs (e.g., `SimulationPhase`) that implement `LoadingDagPhase`.
//! - **Nodes:** Marker component types (e.g., `LoadBlocks`) that represent a distinct task.
//!   Each node is spawned as a dedicated ECS entity when registered.
//! - **Dependencies:** Explicitly map which node types must finish before others can start.
//! - **Targeted Observers:** Loading tasks are implemented as Bevy Observers listening for
//!   `StartNode`. The DAG triggers these observers on the specific entity associated
//!   with that node type.
//! - **Task Lifecycle:** Nodes move from `Pending` -> `Started` -> `Completed`.
//!
//! ## Manual Polling
//!
//! **Important:** The system does not automatically poll tasks. You must manually add the
//! `poll_tasks::<P>` system to the appropriate schedule (e.g., `Update`) during your loading
//! states to ensure that `LoadingTaskComponent` results are processed and nodes are marked as finished.
//!
//! ## Example
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use shared::lifecycle::load::*;
//! # #[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
//! # enum MyState { #[default] Loading }
//! # fn handle_assets(_: On<StartNode>) {}
//! # fn handle_registry(_: On<StartNode>) {}
//!
//! pub struct MyPhase;
//! impl LoadingDagPhase for MyPhase { const PHASE_NAME: &'static str = "MyPhase"; }
//!
//! #[derive(Component)] pub struct LoadAssets;
//! #[derive(Component)] pub struct LoadRegistry;
//!
//! fn setup(mut app: &mut App) {
//!     app.configure_loading_phase::<MyPhase>()
//!         .add_node(LoadAssets, handle_assets)
//!         .add_node(LoadRegistry, handle_registry)
//!         // Registry depends on Assets
//!         .add_dependency(LoadRegistry, LoadAssets);
//!
//!     // Kickoff the loading phase
//!     app.add_systems(OnEnter(MyState::Loading), kickoff_loading_phase::<MyPhase>);
//!
//!     // IMPORTANT: You must manually register the polling system if your loading tasks are asynchronous
//!     app.add_systems(Update, poll_tasks::<MyPhase>.run_if(in_state(MyState::Loading)));
//! }
//! ```

mod components;
mod resource;
mod systems;

pub use components::*;
pub use resource::*;
pub use systems::*;

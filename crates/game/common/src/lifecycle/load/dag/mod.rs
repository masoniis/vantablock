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
//! ## Task Polling
//!
//! Asynchronous tasks spawned with `LoadingTaskComponent` are automatically polled by
//! `poll_all_loading_tasks`, which is registered in the shared lifecycle state plugin.
//!
//! ## Example
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use common::lifecycle::load::*;
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
//! fn setup(app: &mut App) {
//!     app.configure_loading_phase::<MyPhase>()
//!         .add_node(LoadAssets, handle_assets)
//!         .add_node(LoadRegistry, handle_registry)
//!         // Registry depends on Assets
//!         .add_dependency(LoadRegistry, LoadAssets);
//!
//!     // Kickoff the loading phase
//!     app.add_systems(OnEnter(MyState::Loading), kickoff_loading_phase::<MyPhase>);
//! }
//! ```

mod components;
mod resource;
mod systems;

pub use components::*;
pub use resource::*;
pub use systems::*;

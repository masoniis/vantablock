use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::Task;

#[derive(Component)]
pub struct LoadingTaskComponent(pub Task<CommandQueue>);

/// A trait that defines the requirements for a type to be used as a node identifier
/// in the asynchronous loading DAG.
///
/// Types implementing this trait (usually enums) serve three purposes:
/// 1. **Unique Identifiers:** Each variant represents a distinct task in the loading graph.
/// 2. **Phase Markers:** The enum type itself distinguishes between different loading phases
///    (e.g., `AppStartupLoadingPhase` vs `SimulationLoadingPhase`).
/// 3. **Component Filters:** Each variant is attached to its own entity as a component,
///    allowing systems and observers to query for specific tasks.
pub trait LoadingDagPhase:
    Component + Reflect + Copy + Eq + std::hash::Hash + Send + Sync + 'static
{
}

/// Automatically implement LoadingDagPhase for any type that meets the criteria.
impl<T: Component + Reflect + Copy + Eq + std::hash::Hash + Send + Sync + 'static> LoadingDagPhase
    for T
{
}

/// Triggered when a loading node is ready to begin its task.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct StartNode<N: LoadingDagPhase>(pub N);

/// Triggered when a loading node has successfully completed its task.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct NodeFinished<N: LoadingDagPhase>(pub N);

/// Triggered when an entire loading phase has completed all its nodes.
#[derive(Event, Debug, Clone, Copy, Reflect)]
pub struct PhaseFinished<N: LoadingDagPhase>(pub std::marker::PhantomData<N>);

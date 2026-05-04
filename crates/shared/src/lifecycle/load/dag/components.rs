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
    Component + Reflect + Copy + Eq + std::hash::Hash + Send + Sync + std::fmt::Debug + 'static
{
    /// A human-readable name for this loading phase, used in logs and debugging.
    const PHASE_NAME: &'static str;
}

/// Triggered when a loading phase node is ready to begin its task.
#[derive(EntityEvent, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct StartNode<P: LoadingDagPhase> {
    pub entity: Entity,
    pub node: P,
}

/// Triggered when a loading phase node has successfully completed its task.
#[derive(EntityEvent, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct NodeFinished<P: LoadingDagPhase> {
    pub entity: Entity,
    pub node: P,
}

/// Triggered when an entire loading phase has completed all its nodes.
#[derive(Event, Debug, Clone, Copy, Reflect)]
pub struct PhaseFinished<P: LoadingDagPhase>(pub std::marker::PhantomData<P>);

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::Task;
use std::any::TypeId;

#[derive(Component)]
pub struct LoadingTaskComponent(pub Task<CommandQueue>);

/// A trait that defines the requirements for a type to be used as a marker
/// for a specific loading phase (e.g., SimulationPhase, AppStartupLoadingPhase).
pub trait LoadingDagPhase: Send + Sync + 'static {
    /// A human-readable name for this loading phase, used in logs and debugging.
    const PHASE_NAME: &'static str;
}

/// Triggered when a loading phase node is ready to begin its task.
///
/// This is triggered on the entity representing the node.
#[derive(EntityEvent, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct StartNode(pub Entity);

/// Triggered when a loading phase node has successfully completed its task.
#[derive(Event, Debug, Clone, Reflect)]
pub struct NodeCompleted {
    pub node_type: TypeId,
    pub node_name: String,
}

impl NodeCompleted {
    /// Helper method to construct a NodeCompleted event for a specific type.
    pub fn of<T: 'static>() -> Self {
        Self {
            node_type: TypeId::of::<T>(),
            node_name: std::any::type_name::<T>()
                .split("::")
                .last()
                .unwrap_or("Unknown")
                .to_string(),
        }
    }
}

/// Triggered when an entire loading phase has completed all its nodes.
#[derive(Event, Debug, Clone, Copy, Reflect)]
pub struct PhaseFinished<P: LoadingDagPhase>(pub std::marker::PhantomData<P>);

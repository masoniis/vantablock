use bevy::ecs::prelude::*;
use bevy::ecs::world::CommandQueue;
use bevy::tasks::Task;

#[derive(Component)]
pub struct LoadingTaskComponent(pub Task<CommandQueue>);

/// A loading phase for app startup. This phase should include
/// essential rendering tasks that must be done before the first
/// pixel of the startup screen displays. Anything else should be
/// done during the actual startup screen instead.
#[derive(Component)]
pub struct AppStartupLoadingPhase;

/// A loading phase for all tasks necessary to complete before the
/// simulation itself starts running.
#[derive(Component)]
pub struct SimulationLoadingPhase;

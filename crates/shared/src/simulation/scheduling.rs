use bevy::ecs::prelude::SystemSet;

/// The sets for the fixed timestep schedule.
/// Used to strictly order logic within Bevy's native `FixedUpdate` schedule.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum FixedUpdateSet {
    /// Handle state transitions and other pre-logic tasks.
    PreUpdate,
    /// The main sim logic: player movement, AI, block breaking, etc.
    MainLogic,
}

/// Systems that prepare data for the render world.
/// These typically run at the very end of Bevy's native `PostUpdate` schedule.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct RenderPrepSet;

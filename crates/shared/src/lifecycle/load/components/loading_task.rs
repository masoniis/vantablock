use bevy::ecs::prelude::*;
use bevy::ecs::world::CommandQueue;
use bevy::tasks::Task;

#[derive(Component)]
pub struct LoadingTaskComponent(pub Task<CommandQueue>);

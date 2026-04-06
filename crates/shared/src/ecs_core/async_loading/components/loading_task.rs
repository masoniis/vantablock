use bevy::ecs::prelude::*;
use crossbeam::channel::Receiver;

pub type TaskResultCallback = Box<dyn FnOnce(&mut Commands) + Send>;

#[derive(Component)]
pub struct SimulationWorldLoadingTaskComponent {
    pub receiver: Receiver<TaskResultCallback>,
}

#[derive(Component)]
pub struct RenderWorldLoadingTaskComponent {
    pub receiver: Receiver<TaskResultCallback>,
}

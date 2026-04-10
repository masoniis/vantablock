use bevy::ecs::prelude::*;
use crossbeam::channel::Receiver;

pub type TaskResultCallback = Box<dyn FnOnce(&mut Commands) + Send>;

// TODO: no longer need with bevy tasks

#[derive(Component)]
pub struct SimulationWorldLoadingTaskComponent {
    pub receiver: Receiver<TaskResultCallback>,
}

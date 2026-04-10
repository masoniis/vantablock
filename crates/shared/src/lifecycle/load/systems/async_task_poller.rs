use crate::{
    lifecycle::load::{LoadingTaskComponent, LoadingTracker},
    prelude::*,
};
use bevy::tasks::block_on;
use bevy::{ecs::prelude::*, tasks::poll_once};

/// Polls simulation-specific tasks and updates the shared `LoadingTracker`.
#[instrument(skip_all)]
pub fn poll_simulation_loading_tasks(
    // input
    mut tasks: Query<(Entity, &mut LoadingTaskComponent)>,

    // output
    mut commands: Commands,
    loading_tracker: Option<Res<LoadingTracker>>,
) {
    let Some(loading_tracker) = loading_tracker else {
        return;
    };

    let mut remaining_tasks = 0;

    for (entity, mut task) in &mut tasks {
        if let Some(mut command_queue) = block_on(poll_once(&mut task.0)) {
            commands.append(&mut command_queue);
            commands.entity(entity).despawn();
        } else {
            remaining_tasks += 1;
        }
    }

    if remaining_tasks == 0 {
        loading_tracker.set_simulation_ready(true);
    } else {
        loading_tracker.set_simulation_ready(false);
    }
}

use crate::{
    ecs_core::async_loading::{LoadingTracker, loading_task::SimulationWorldLoadingTaskComponent},
    prelude::*,
};
use bevy::ecs::prelude::*;
use crossbeam::channel::TryRecvError;

/// Polls simulation-specific tasks and updates the shared `LoadingTracker`.
#[instrument(skip_all)]
pub fn poll_simulation_loading_tasks(
    // Input
    mut tasks: Query<(Entity, &mut SimulationWorldLoadingTaskComponent)>,

    // Output (updated states)
    mut commands: Commands,
    loading_tracker: Option<Res<LoadingTracker>>,
) {
    let Some(loading_tracker) = loading_tracker else {
        return;
    };

    // Local counter to track tasks that are still running this frame.
    // This correctly handles the case where 0 tasks were spawned.
    let mut remaining_tasks = 0;

    for (entity, task) in &mut tasks {
        match task.receiver.try_recv() {
            Ok(callback) => {
                callback(&mut commands);
                commands.entity(entity).despawn();
            }
            Err(TryRecvError::Empty) => {
                // Task is still working, count it.
                remaining_tasks += 1;
            }
            Err(TryRecvError::Disconnected) => {
                eprintln!("Task failed: Channel disconnected!");
                commands.entity(entity).despawn();
            }
        }
    }

    // Use the local counter to determine if all tasks are done.
    if remaining_tasks == 0 && !loading_tracker.is_ready() {
        debug!(
            target: "async_tasks",
            "[POLL] All tasks are complete. Marking simulation ready.",
        );
        loading_tracker.set_simulation_ready(true);
    }
}

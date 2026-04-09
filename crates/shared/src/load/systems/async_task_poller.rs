use crate::{
    load::{LoadingTracker, SimulationWorldLoadingTaskComponent},
    prelude::*,
};
use bevy::ecs::prelude::*;
use crossbeam::channel::TryRecvError;

/// Polls simulation-specific tasks and updates the shared `LoadingTracker`.
#[instrument(skip_all)]
pub fn poll_simulation_loading_tasks(
    // input
    mut tasks: Query<(Entity, &mut SimulationWorldLoadingTaskComponent)>,

    // output
    mut commands: Commands,
    loading_tracker: Option<Res<LoadingTracker>>,
) {
    let Some(loading_tracker) = loading_tracker else {
        return;
    };

    let mut remaining_tasks = 0;

    for (entity, task) in &mut tasks {
        match task.receiver.try_recv() {
            Ok(callback) => {
                callback(&mut commands);
                commands.entity(entity).despawn();
            }
            Err(TryRecvError::Empty) => {
                remaining_tasks += 1;
            }
            Err(TryRecvError::Disconnected) => {
                // This could happen if a thread panicked or dropped the sender.
                // It shouldn't happen during normal execution.
                warn!("Simulation task failed: Channel disconnected!");
                commands.entity(entity).despawn();
            }
        }
    }

    if remaining_tasks == 0 {
        loading_tracker.set_simulation_ready(true);
    } else {
        loading_tracker.set_simulation_ready(false);
    }
}

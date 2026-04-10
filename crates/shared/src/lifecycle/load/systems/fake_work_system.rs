use crate::{
    lifecycle::load::components::{LoadingTaskComponent, SimulationPhase},
    prelude::*,
};
use bevy::{
    ecs::system::Commands,
    ecs::world::{CommandQueue, World},
    tasks::AsyncComputeTaskPool,
};
use rand::random_range;
use std::{thread, time::Duration};

#[instrument(skip_all)]
pub fn start_fake_work_system(mut commands: Commands) {
    let entity = commands.spawn_empty().id();
    let task = AsyncComputeTaskPool::get().spawn(async move {
        const WORK_DURATION: u64 = 3;
        for i in 1..=WORK_DURATION {
            info!(
                "[BACKGROUND {}] Fake working... step {}/{}",
                entity, i, WORK_DURATION
            );

            let sleep_time = random_range(0.25..0.75);
            thread::sleep(Duration::from_secs_f32(sleep_time));
        }
        info!("[BACKGROUND {}] Fake work finished!", entity);

        let mut queue = CommandQueue::default();
        queue.push(move |_: &mut World| {
            info!(
                "[MAIN THREAD {}] Applying fake work result to the world.",
                entity
            );
        });
        queue
    });

    commands
        .entity(entity)
        .insert((LoadingTaskComponent(task), SimulationPhase));
}

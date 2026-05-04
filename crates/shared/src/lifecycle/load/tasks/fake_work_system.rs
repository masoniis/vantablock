use crate::prelude::*;
use bevy::{
    ecs::world::{CommandQueue, World},
    tasks::{AsyncComputeTaskPool, Task},
};
use rand::random_range;
use std::{thread, time::Duration};

#[instrument(skip_all)]
pub fn start_fake_work_system() -> Task<CommandQueue> {
    AsyncComputeTaskPool::get().spawn(async move {
        const WORK_DURATION: u64 = 3;
        for i in 1..=WORK_DURATION {
            info!("[BACKGROUND] Fake working... step {}/{}", i, WORK_DURATION);

            let sleep_time = random_range(0.25..0.75);
            thread::sleep(Duration::from_secs_f32(sleep_time));
        }
        info!("[BACKGROUND] Fake work finished!");

        let mut queue = CommandQueue::default();
        queue.push(move |_world: &mut World| {
            info!("[MAIN THREAD] Applying fake work result to the world.");
        });
        queue
    })
}

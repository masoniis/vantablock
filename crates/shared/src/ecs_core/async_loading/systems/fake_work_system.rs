use crate::{
    ecs_core::async_loading::loading_task::{
        SimulationWorldLoadingTaskComponent, TaskResultCallback,
    },
    prelude::*,
};
use bevy::ecs::system::Commands;
use crossbeam::channel::{Receiver, Sender, unbounded};
use rand::random_range;
use std::{thread, time::Duration};

#[instrument(skip_all)]
pub fn start_fake_work_system(mut commands: Commands) {
    let (sender, receiver): (Sender<TaskResultCallback>, Receiver<TaskResultCallback>) =
        unbounded();

    let entity = commands.spawn_empty().id();
    rayon::spawn(move || {
        const WORK_DURATION: u64 = 3;
        for i in 1..=WORK_DURATION {
            info!(
                "[BACKGROUND {}] Fake working... step {}/{}",
                entity, i, WORK_DURATION
            );

            let sleep_time = random_range(0.25..0.8);
            thread::sleep(Duration::from_secs_f32(sleep_time));
        }
        info!("[BACKGROUND {}] Fake work finished!", entity);

        let callback: TaskResultCallback = Box::new(move |_: &mut Commands| {
            info!(
                "[MAIN THREAD {}] Applying fake work result to the world.",
                entity
            );
        });

        sender.send(callback).expect("Failed to send task result");
    });

    commands
        .entity(entity)
        .insert(SimulationWorldLoadingTaskComponent { receiver });
}

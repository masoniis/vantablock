use crate::{
    prelude::*,
    simulation_world::{time::FrameClock, SimulationSchedule},
};
use bevy::ecs::prelude::*;
use std::time::Duration;

#[derive(Resource, Debug)]
pub struct SimulationTick {
    pub tick: u64,

    pub tick_rate: f32,
    pub tick_duration: Duration,
}

impl Default for SimulationTick {
    fn default() -> Self {
        let tick_rate = 16.0;
        let tick_duration = Duration::from_secs_f32(1.0 / tick_rate);
        SimulationTick {
            tick: 0,
            tick_rate,
            tick_duration,
        }
    }
}

/// An exclusive system that updates the simulation tick and runs
/// the fixed update schedule if a tick has come.
#[instrument(skip_all)]
pub fn run_fixed_update_schedule(world: &mut World) {
    let tick_duration = world.resource::<SimulationTick>().tick_duration;

    while world.resource::<FrameClock>().accumulator >= tick_duration {
        world
            .resource_mut::<FrameClock>()
            .decrement_accumulator_tick(tick_duration);

        world.resource_mut::<SimulationTick>().tick += 1;

        world.run_schedule(SimulationSchedule::FixedUpdate);
    }

    // calculate interpolation alpha after processing fixed updates
    let mut frame_clock = world.resource_mut::<FrameClock>();
    let alpha = frame_clock.accumulator.as_secs_f32() / tick_duration.as_secs_f32();
    frame_clock.alpha = alpha;
}

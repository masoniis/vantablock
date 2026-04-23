pub mod frame_clock;
pub mod simulation_tick;
pub mod world_clock;

pub use frame_clock::FrameClock;
pub use world_clock::WorldClockResource;

// INFO: ---------------------
//         Time plugin
// ---------------------------

use crate::lifecycle::state::SimulationState;
use crate::simulation::time::{
    frame_clock::update_frame_clock_system, simulation_tick::SimulationTick,
    simulation_tick::run_fixed_update_schedule, world_clock::update_world_clock_system,
};
use bevy::{
    app::{App, FixedUpdate, Plugin, PreUpdate, Update},
    ecs::prelude::IntoScheduleConfigs,
    state::condition::in_state,
};

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        // Maintain a clock that tracks frame time and provides timing info
        app.insert_resource(FrameClock::default()).add_systems(
            PreUpdate,
            (update_frame_clock_system).run_if(in_state(SimulationState::Running)),
        );

        // Trigger the simulation ticks when appropriate
        app.insert_resource(SimulationTick::default()).add_systems(
            Update,
            run_fixed_update_schedule.run_if(in_state(SimulationState::Running)),
        );

        // Maintain world clock that depends on ticks rather that frames
        app.insert_resource(WorldClockResource::default())
            .add_systems(FixedUpdate, update_world_clock_system);
    }
}

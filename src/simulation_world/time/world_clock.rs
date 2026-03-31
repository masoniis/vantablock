use crate::prelude::*;
use crate::simulation_world::time::simulation_tick::SimulationTick;
use bevy::ecs::prelude::*;
use std::time::Duration;

/// Number of seconds it takes for a day/night cycle to complete
pub const SECONDS_IN_A_DAY: f32 = 600.0;
/// number of seconds to jump when using jump forward/backward commands
const JUMP_CLOCK_DISTANCE: f32 = 30.0;

/// A resource that tracks the in-game date and time.
#[derive(Resource, Debug)]
pub struct WorldClockResource {
    /// The total number of full days that have passed since the world began.
    pub total_days: u64,
    /// The current time within the 24-hour cycle.
    pub time_of_day: Duration,
    /// The duration of a full in-game day.
    pub day_duration: Duration,
}

impl Default for WorldClockResource {
    fn default() -> Self {
        Self {
            total_days: 0,
            time_of_day: Duration::from_secs_f32(SECONDS_IN_A_DAY * 0.70),
            day_duration: Duration::from_secs_f32(SECONDS_IN_A_DAY),
        }
    }
}

impl WorldClockResource {
    /// Returns the current time of day as a value from 0.0 (midnight) to 1.0 (next midnight).
    pub fn day_night_cycle_value(&self) -> f32 {
        self.time_of_day.as_secs_f32() / SECONDS_IN_A_DAY
    }
}

// INFO: ----------------------------
//         update world clock
// ----------------------------------

/// A system that runs every tick to advance the in-game calendar.
#[instrument(skip_all)]
pub fn update_world_clock_system(
    // Input
    sim_tick: Res<SimulationTick>,

    // Output
    mut world_clock: ResMut<WorldClockResource>,
) {
    world_clock.time_of_day += sim_tick.tick_duration;

    if world_clock.time_of_day >= world_clock.day_duration {
        let day_dur = world_clock.day_duration;
        world_clock.time_of_day -= day_dur;
        world_clock.total_days += 1;
    }
}

/// A system that jumps the in game world clock forward
///
/// Should run before update_world_clock_system
#[instrument(skip_all)]
pub fn jump_world_clock_forward_system(
    // Output
    mut world_clock: ResMut<WorldClockResource>,
) {
    world_clock.time_of_day += Duration::from_secs_f32(JUMP_CLOCK_DISTANCE);

    if world_clock.time_of_day >= world_clock.day_duration {
        let day_dur = world_clock.day_duration;
        world_clock.time_of_day -= day_dur;
        world_clock.total_days += 1;
    }
}

/// A system that jumps the in game world clock backwards
///
/// Should run before update_world_clock_system
#[instrument(skip_all)]
pub fn jump_world_clock_backwards_system(
    // Output
    mut world_clock: ResMut<WorldClockResource>,
) {
    let jump_dist = Duration::from_secs_f32(JUMP_CLOCK_DISTANCE);

    // subtract 30 seconds, wrapping day if needed
    if let Some(new_time) = world_clock.time_of_day.checked_sub(jump_dist) {
        world_clock.time_of_day = new_time;
    } else if world_clock.total_days > 0 {
        world_clock.total_days -= 1;
        let underflow_by = jump_dist - world_clock.time_of_day;
        world_clock.time_of_day = world_clock.day_duration - underflow_by;
    } else {
        world_clock.time_of_day = Duration::ZERO;
    }
}

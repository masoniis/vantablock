use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Number of seconds it takes for a day/night cycle to complete
pub const SECONDS_IN_A_DAY: f32 = 600.0;
/// Number of seconds the time-jump keys move the clock by.
pub const JUMP_CLOCK_DISTANCE: f32 = 30.0;

/// A resource that tracks the in-game date and time.
///
/// The server's clock is authoritative: clients run their own copy between syncs and snap it to
/// every `ServerMessage::SyncTime`.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WorldClockResource {
    /// The total number of full days that have passed since the world began.
    pub total_days: u64,
    /// The current time within the 24-hour cycle.
    pub time_of_day: Duration,
    /// The duration of a full in-game day.
    pub day_duration: Duration,
    /// Whether the clock is stopped.
    pub paused: bool,
}

impl Default for WorldClockResource {
    fn default() -> Self {
        Self {
            total_days: 0,
            time_of_day: Duration::from_secs_f32(SECONDS_IN_A_DAY * 0.70),
            day_duration: Duration::from_secs_f32(SECONDS_IN_A_DAY),
            paused: false,
        }
    }
}

impl WorldClockResource {
    /// Returns the current time of day as a value from 0.0 (midnight) to 1.0 (next midnight).
    pub fn day_night_cycle_value(&self) -> f32 {
        self.time_of_day.as_secs_f32() / SECONDS_IN_A_DAY
    }

    /// Moves the clock by `seconds` (negative goes back), rolling over into the next or previous
    /// day. It never goes before the start of day zero.
    pub fn jump(&mut self, seconds: f32) {
        // f64 so per-tick advances stay exact deep into a long-running world
        let day = self.day_duration.as_secs_f64();
        let total =
            (self.total_days as f64 * day + self.time_of_day.as_secs_f64() + seconds as f64)
                .max(0.0);
        self.total_days = (total / day).floor() as u64;
        self.time_of_day = Duration::from_secs_f64(total.rem_euclid(day));
    }

    /// Sets the time within the current day, as a fraction from 0.0 (midnight) to 1.0.
    pub fn set_time_of_day(&mut self, fraction: f32) {
        self.time_of_day = self.day_duration.mul_f32(fraction.rem_euclid(1.0));
    }
}

// INFO: ----------------------------
//         update world clock
// ----------------------------------

/// A system that runs every fixed tick to advance the in-game calendar.
#[instrument(skip_all)]
pub fn update_world_clock_system(
    // Input
    time: Res<Time<Fixed>>,
    // Output
    mut world_clock: ResMut<WorldClockResource>,
) {
    if world_clock.paused {
        return;
    }
    let delta = time.delta_secs();
    world_clock.jump(delta);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jumps_roll_over_days_both_ways() {
        let mut clock = WorldClockResource {
            total_days: 1,
            time_of_day: Duration::from_secs(10),
            ..Default::default()
        };

        clock.jump(-30.0);
        assert_eq!(clock.total_days, 0);
        assert!((clock.time_of_day.as_secs_f32() - (SECONDS_IN_A_DAY - 20.0)).abs() < 0.01);

        clock.jump(40.0);
        assert_eq!(clock.total_days, 1);
        assert!((clock.time_of_day.as_secs_f32() - 20.0).abs() < 0.01);

        // never before the start of day zero
        clock.jump(-10_000.0);
        assert_eq!((clock.total_days, clock.time_of_day), (0, Duration::ZERO));
    }
}

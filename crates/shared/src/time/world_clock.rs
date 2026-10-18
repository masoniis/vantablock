use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Number of seconds it takes for a day/night cycle to complete
pub const SECONDS_IN_A_DAY: f32 = 600.0;

/// A resource that tracks world time: a monotonic animation clock and the day/night cycle.
#[derive(Resource, Debug, Clone)]
pub struct WorldClockResource {
    /// Total world time. Drives animation and never jumps or pauses.
    pub elapsed: Duration,
    /// The total number of full days that have passed since the world began.
    pub total_days: u64,
    /// The current time within the day cycle.
    pub time_of_day: Duration,
    /// The duration of a full in-game day.
    pub day_duration: Duration,
    /// Freezes the day/night cycle (but not animation).
    pub day_cycle_paused: bool,
}

impl Default for WorldClockResource {
    fn default() -> Self {
        Self {
            elapsed: Duration::ZERO,
            total_days: 0,
            time_of_day: Duration::from_secs_f32(SECONDS_IN_A_DAY * 0.36),
            day_duration: Duration::from_secs_f32(SECONDS_IN_A_DAY),
            day_cycle_paused: false,
        }
    }
}

impl WorldClockResource {
    /// Returns the current time of day as a value from 0.0 (midnight) to 1.0 (next midnight).
    pub fn day_night_cycle_value(&self) -> f32 {
        self.time_of_day.as_secs_f32() / self.day_duration.as_secs_f32()
    }

    /// The time used to evaluate animation.
    pub fn animation_seconds(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    pub fn advance(&mut self, delta: Duration) {
        self.elapsed += delta;
        if !self.day_cycle_paused {
            self.shift_time_of_day(delta.as_secs_f32());
        }
    }

    /// Moves the time of day by `seconds` (negative rewinds), wrapping across days.
    pub fn shift_time_of_day(&mut self, seconds: f32) {
        let day = self.day_duration.as_secs_f64();
        let total = self.total_days as f64 * day + self.time_of_day.as_secs_f64() + seconds as f64;
        let total = total.max(0.0);
        self.total_days = (total / day) as u64;
        self.time_of_day = Duration::from_secs_f64(total % day);
    }
}

// INFO: ----------------------------
//         update world clock
// ----------------------------------

/// Advances the world clock by the schedule's delta: ticks in `FixedUpdate`, frames in `Update`.
#[instrument(skip_all)]
pub fn advance_world_clock_system(time: Res<Time>, mut world_clock: ResMut<WorldClockResource>) {
    world_clock.advance(time.delta());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_of_day_wraps_both_ways() {
        let mut clock = WorldClockResource {
            time_of_day: Duration::from_secs_f32(SECONDS_IN_A_DAY - 10.0),
            ..Default::default()
        };
        clock.shift_time_of_day(30.0);
        assert_eq!(clock.total_days, 1);
        assert!((clock.time_of_day.as_secs_f32() - 20.0).abs() < 1e-3);

        clock.shift_time_of_day(-40.0);
        assert_eq!(clock.total_days, 0);
        assert!((clock.time_of_day.as_secs_f32() - (SECONDS_IN_A_DAY - 20.0)).abs() < 1e-3);
    }

    #[test]
    fn pausing_freezes_the_day_but_not_animation() {
        let mut clock = WorldClockResource {
            day_cycle_paused: true,
            ..Default::default()
        };
        let before = clock.time_of_day;
        clock.advance(Duration::from_secs(5));
        assert_eq!(clock.time_of_day, before);
        assert_eq!(clock.elapsed, Duration::from_secs(5));
    }
}

use crate::prelude::*;
use bevy::ecs::prelude::*;
use std::time::{Duration, Instant};

const FPS_SMOOTHING_FACTOR: f32 = 0.025;

// INFO: ------------------------
//         clock resource
// ------------------------------

/// A resource that tracks frame timing information.
///
/// Due to its nature, it runs every every single frame.
#[derive(Resource)]
pub struct FrameClock {
    /// The instant the clock was created.
    startup: Instant,
    /// The instant of the last update.
    last_update: Instant,

    /// Time elapsed since the last update.
    pub delta: Duration,
    /// Total time elapsed since the app (clock) started. This is NEVER scaled or paused.
    pub elapsed: Duration,

    /// Accumulates time for fixed updates. When a new tick occurs,
    /// the accumulator loses `TICK_DURATION` amount of time.
    pub accumulator: Duration,

    /// Interpolation factor (0.0 to 1.0) for smooth rendering.
    pub alpha: f32,

    /// The smoothed frames per second (FPS) value.
    pub smoothed_fps: f32,
}

impl Default for FrameClock {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            startup: now,
            last_update: now,
            elapsed: Duration::ZERO,
            delta: Duration::ZERO,
            accumulator: Duration::ZERO,
            smoothed_fps: 69.0,
            alpha: 0.0,
        }
    }
}

impl FrameClock {
    /// Updates all timing information based on the current instant.
    pub fn update_all(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update);

        self.last_update = now;
        self.accumulator += delta;
        self.delta = delta;
        self.elapsed = now.duration_since(self.startup);
        self.update_fps();

        debug!(target : "fps", "FPS: {:?}", self.smoothed_fps);
    }

    /// Updates the smoothed FPS using an exponential moving average.
    fn update_fps(&mut self) {
        let current_raw_fps = if self.delta.as_secs_f32() > 0.0 {
            1.0 / self.delta.as_secs_f32()
        } else {
            0.0
        };

        self.smoothed_fps = (current_raw_fps * FPS_SMOOTHING_FACTOR)
            + (self.smoothed_fps * (1.0 - FPS_SMOOTHING_FACTOR));
    }

    /// Decrements the accumulator by the amount of a single tick
    pub fn decrement_accumulator_tick(&mut self, amount: Duration) {
        if self.accumulator >= amount {
            self.accumulator -= amount;
        } else {
            self.accumulator = Duration::ZERO;
        }
    }
}

// INFO: -----------------------
//         update system
// -----------------------------

/// A system that updates the `FrameClock` resource every frame.
#[instrument(skip_all)]
pub fn update_frame_clock_system(mut clock: ResMut<FrameClock>) {
    clock.update_all();
}

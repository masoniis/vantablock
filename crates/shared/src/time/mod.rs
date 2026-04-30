mod frame_clock;
mod world_clock;

pub use frame_clock::FrameClock;
pub use world_clock::{SECONDS_IN_A_DAY, WorldClockResource};

/// Const for configuration of fixed update tick rate and network tick rate
pub const TICKS_PER_SECOND: f64 = 20.0;

// INFO: ---------------------
//         time plugin
// ---------------------------

use crate::time::{frame_clock::update_frame_clock_system, world_clock::update_world_clock_system};
use bevy::{
    app::{App, FixedUpdate, Plugin, Update},
    time::{Fixed, Time},
};

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        // configure bevy fixed update tick rate
        app.insert_resource(Time::<Fixed>::from_hz(TICKS_PER_SECOND));

        app.insert_resource(WorldClockResource::default())
            .add_systems(FixedUpdate, update_world_clock_system);

        app.insert_resource(FrameClock::default())
            .add_systems(Update, update_frame_clock_system);
    }
}

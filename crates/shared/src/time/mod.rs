mod world_clock;

pub use world_clock::{SECONDS_IN_A_DAY, WorldClockResource, advance_world_clock_system};

/// Const for configuration of fixed update tick rate
pub const TICKS_PER_SECOND: f64 = 20.0;

// INFO: ---------------------
//         time plugin
// ---------------------------

use bevy::{
    app::{App, Plugin},
    time::{Fixed, Time},
};

/// Configures the fixed tick rate and provides the world clock.
///
/// Each side decides how the clock advances: the server ticks it authoritatively in
/// `FixedUpdate`, while clients advance it every frame for smooth animation and snap to the
/// server's periodic syncs.
pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        // configure bevy fixed update tick rate
        app.insert_resource(Time::<Fixed>::from_hz(TICKS_PER_SECOND));
        app.init_resource::<WorldClockResource>();
    }
}

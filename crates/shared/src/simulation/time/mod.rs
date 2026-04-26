pub mod frame_clock;
pub mod world_clock;

pub use frame_clock::FrameClock;
pub use world_clock::WorldClockResource;

// INFO: ---------------------
//         time plugin
// ---------------------------

use crate::simulation::time::{
    frame_clock::update_frame_clock_system, world_clock::update_world_clock_system,
};
use bevy::app::{App, FixedUpdate, Plugin, Update};

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldClockResource::default())
            .add_systems(FixedUpdate, update_world_clock_system);

        app.insert_resource(FrameClock::default())
            .add_systems(Update, update_frame_clock_system);
    }
}

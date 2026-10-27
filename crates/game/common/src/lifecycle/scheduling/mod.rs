pub mod sets;

pub use sets::FixedUpdateSet;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;

pub struct SharedSchedulingPlugin;

impl Plugin for SharedSchedulingPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).chain(),
        );
    }
}

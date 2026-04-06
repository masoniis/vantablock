pub mod camera;

pub use camera::*;

// INFO: -----------------------
//         player plugin
// -----------------------------

use bevy::app::{App, Plugin};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CameraPlugin,
            shared::simulation::player::actions::ActionPlugin,
        ));
    }
}

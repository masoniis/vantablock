pub mod smoothing;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct NetworkGraphicsPlugin;

impl Plugin for NetworkGraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                smoothing::update_logical_position_smoothing,
                smoothing::update_player_look_smoothing,
            ),
        );
    }
}

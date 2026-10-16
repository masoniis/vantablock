pub mod components;
pub mod movement;
mod player_action;

pub use components::*;
pub use player_action::PlayerAction;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::input::leafwing::InputPlugin;
use movement::shared_player_movement_system;

/// A plugin that handles shared player logic.
pub struct SharedPlayerPlugin;

impl Plugin for SharedPlayerPlugin {
    fn build(&self, app: &mut App) {
        // handle leafwing inputs via lightyear
        app.add_plugins(InputPlugin::<PlayerAction>::default());

        // NOTE: this resource only exists as a sink, without it
        // leafwing will complain when no players exist since there
        // is no action states to  handle
        app.init_resource::<ActionState<PlayerAction>>();

        app.add_systems(FixedUpdate, shared_player_movement_system);
    }
}

pub mod actions;
pub mod components;
pub mod player_action;

pub use player_action::PlayerAction;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::input::leafwing::InputPlugin;

/// A plugin that handles shared player logic.
pub struct SharedPlayerPlugin;

impl Plugin for SharedPlayerPlugin {
    fn build(&self, app: &mut App) {
        // handle leafwing inputs via lightyear
        app.add_plugins(InputPlugin::<PlayerAction>::default());
    }
}

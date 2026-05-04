pub mod enums;
mod systems;

pub use systems::transition_to;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::enums::AppState;
use bevy::{
    prelude::{App, Plugin},
    state::app::AppExtStates,
};

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>();
    }
}

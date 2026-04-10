pub mod enums;
pub mod lifecycle;

pub use enums::*;
pub use lifecycle::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{
    prelude::{App, Plugin},
    state::app::AppExtStates,
};

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ClientAppState>()
            .add_sub_state::<ClientGameState>();
    }
}

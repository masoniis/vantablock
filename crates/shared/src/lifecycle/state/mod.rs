pub mod enums;
pub mod systems;

pub use enums::{AppState, SimulationState};
pub use systems::transition_to;

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
        app.init_state::<AppState>();
        app.init_state::<SimulationState>();
    }
}

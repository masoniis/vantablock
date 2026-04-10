pub mod enums;
pub mod lifecycle;
pub mod systems;

pub use enums::SimulationState;
pub use lifecycle::SimulationLifecyclePlugin;
pub use systems::transition_to;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{
    prelude::{App, Plugin},
    state::app::AppExtStates,
};
use enums::AppState;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>();
        app.add_plugins(SimulationLifecyclePlugin);
    }
}

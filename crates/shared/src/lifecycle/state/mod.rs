mod enums;
mod systems;

pub use enums::AppState;
pub use systems::transition_to;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::load::poll_all_loading_tasks;
use bevy::{
    prelude::{App, Plugin, Update},
    state::app::AppExtStates,
};

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(Update, poll_all_loading_tasks);
    }
}

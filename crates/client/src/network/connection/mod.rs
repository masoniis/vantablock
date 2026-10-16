mod initiate;
pub mod resources;
mod session;

pub use resources::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::ClientLifecycleState;
use bevy::{
    app::{App, Plugin, Update},
    prelude::{IntoScheduleConfigs, OnExit, in_state},
};

pub(crate) struct NetworkConnectionPlugin;

impl Plugin for NetworkConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(initiate::on_initiate_connection);

        // only an unexpected drop while in game is an error, leaving the game
        // tears the connection down on purpose
        app.add_systems(
            Update,
            session::handle_disconnections.run_if(in_state(ClientLifecycleState::InGame)),
        )
        .add_observer(session::handle_connections);

        app.add_systems(OnExit(ClientLifecycleState::InGame), session::end_session);
    }
}

pub mod local_connection;
pub mod resources;

pub use local_connection::setup_client;
pub use resources::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::InGameState;
use bevy::{
    app::{App, Plugin},
    state::state::OnEnter,
};

pub struct ClientConnectionPlugin;

impl Plugin for ClientConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(InGameState::Connecting), setup_client);

        app.init_resource::<ConnectionSettings>();
    }
}

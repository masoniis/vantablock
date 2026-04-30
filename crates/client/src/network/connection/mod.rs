pub mod resources;

pub use resources::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

mod handle_connection_states;
mod perform_connection;

use bevy::app::{App, Plugin, Update};
use perform_connection::initiate_connection_trigger;

use crate::network::connection::handle_connection_states::{
    handle_connections, handle_disconnections,
};

pub struct ClientConnectionPlugin;

impl Plugin for ClientConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(initiate_connection_trigger);

        app.add_systems(Update, handle_disconnections)
            .add_observer(handle_connections);
    }
}

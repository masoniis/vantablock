mod handle_connections;
mod start_server;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    lifecycle::state::ServerState,
    network::connection::handle_connections::{handle_connections, handle_disconnections},
};
use bevy::prelude::*;
use start_server::start_udp_server;

pub struct ServerConnectionPlugin;

impl Plugin for ServerConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(ServerState::Initializing), start_udp_server)
            .add_observer(handle_connections)
            .add_observer(handle_disconnections);
    }
}

mod listener;
mod session;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::state::ServerState;
use bevy::{
    app::{App, Plugin},
    state::state::OnExit,
};

pub struct ServerConnectionPlugin;

impl Plugin for ServerConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnExit(ServerState::Initializing),
            listener::start_udp_server,
        )
        .add_observer(session::on_client_connect)
        .add_observer(session::on_client_disconnect);
    }
}

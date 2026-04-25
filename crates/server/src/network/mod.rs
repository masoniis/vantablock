pub mod systems;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;
use shared::network::NETWORK_TICK_DURATION;
use std::time::Duration;
use systems::{handle_connections, start_udp_server};

use crate::lifecycle::state::ServerState;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        // lightyear plugins
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
        });

        app.add_systems(OnExit(ServerState::Initializing), start_udp_server)
            .add_observer(handle_connections);
    }
}

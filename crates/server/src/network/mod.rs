// INFO: ---------------------------
//         plugin definition
// ---------------------------------

pub mod systems;

use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;
use shared::network::NETWORK_TICK_DURATION;
use std::time::Duration;
use systems::{handle_connections, start_server};

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
        });

        app.add_systems(Startup, start_server)
            .add_observer(handle_connections);
    }
}

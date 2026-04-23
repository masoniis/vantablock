pub const FIXED_TIMESTEP_HZ: f64 = 60.0;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

pub mod systems;

use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;
use std::time::Duration;
use systems::{handle_connections, start_server};

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
        });

        app.add_systems(Startup, start_server)
            .add_observer(handle_connections);
    }
}

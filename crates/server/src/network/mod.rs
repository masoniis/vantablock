pub mod connection;
pub mod receive;
pub mod send;
pub mod types;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;
use shared::network::{NETWORK_TICK_DURATION, SharedNetworkPlugin};
use std::time::Duration;

use types::MessageTimer;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // lightyear plugins
            ServerPlugins {
                tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
            },
            // network sub-plugins
            connection::ServerConnectionPlugin,
            receive::ServerIngressPlugin,
            send::ServerEgressPlugin,
        ));

        // (protocol) must be added AFTER lightyear plugin
        app.add_plugins(SharedNetworkPlugin);

        app.insert_resource(MessageTimer(Timer::from_seconds(1.0, TimerMode::Repeating)));
    }
}

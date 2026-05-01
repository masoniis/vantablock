pub mod systems;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;
use shared::network::{NETWORK_TICK_DURATION, SharedNetworkPlugin};
use std::time::Duration;
use systems::{
    MessageTimer, handle_connections, handle_disconnections, receive_client_messages,
    send_sync_time, start_udp_server,
};

use crate::lifecycle::state::ServerState;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        // lightyear plugins
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
        });

        // (protocl) must be added AFTER lightyear plugin
        app.add_plugins(SharedNetworkPlugin);

        app.insert_resource(MessageTimer(Timer::from_seconds(1.0, TimerMode::Repeating)));

        app.add_systems(OnExit(ServerState::Initializing), start_udp_server)
            .add_systems(Update, (send_sync_time, receive_client_messages))
            .add_observer(handle_connections)
            .add_observer(handle_disconnections);
    }
}

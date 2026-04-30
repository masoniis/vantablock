pub mod connection;
pub mod ingress;
pub mod systems;

pub use ingress::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::client as lightyear_client;
use shared::network::{NETWORK_TICK_DURATION, SharedNetworkPlugin};
use std::time::Duration;
use systems::apply_received_chunk_data_system;

pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // lightyear plugin group
            lightyear_client::ClientPlugins {
                tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
            },
            // client's message handler
            ingress::ClientMessageHandlerPlugin,
            connection::ClientConnectionPlugin,
        ));

        // (protocol) must be added AFTER lightyear plugin
        app.add_plugins(SharedNetworkPlugin);

        app.add_systems(Update, apply_received_chunk_data_system);
    }
}

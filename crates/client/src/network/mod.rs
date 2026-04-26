pub mod local_connection;
pub mod message_handler;
pub mod messages;
pub mod resources;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::state::InGameState;
use crate::network::resources::ConnectionSettings;
use bevy::prelude::*;
use lightyear::prelude::client as lightyear_client;
use local_connection::setup_client;
use shared::network::NETWORK_TICK_DURATION;
use std::time::Duration;

pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionSettings>();

        app.add_plugins((
            // lightyear plugin group
            lightyear_client::ClientPlugins {
                tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
            },
            // client's message handler
            message_handler::ClientMessageHandlerPlugin,
        ));

        app.add_systems(OnEnter(InGameState::Connecting), setup_client);
    }
}

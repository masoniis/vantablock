pub mod local_connection;
pub mod message_handler;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::input::resources::ActionStateResource;
use crate::lifecycle::state::InGameState;
use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;
use local_connection::setup_client;
use shared::network::NETWORK_TICK_DURATION;
use shared::simulation::input::types::SimulationAction;
use std::time::Duration;

use crate::network::local_connection::check_connection_state;

pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
        });

        app.add_systems(
            Update,
            (
                setup_client.run_if(|action_state: Res<ActionStateResource>| {
                    action_state.just_happened(SimulationAction::ToggleChunkBorders)
                }),
                check_connection_state,
            ),
        );

        app.add_systems(OnEnter(InGameState::Connecting), setup_client);

        app.add_plugins(message_handler::ClientMessageHandlerPlugin);
    }
}

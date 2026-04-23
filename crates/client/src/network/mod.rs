pub mod local_connection;
pub mod message_handler;
pub mod resources;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::input::resources::ActionStateResource;
use crate::lifecycle::state::InGameState;
use crate::network::resources::ConnectionSettings;
use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;
use local_connection::setup_client;
use shared::network::NETWORK_TICK_DURATION;
use shared::simulation::input::types::SimulationAction;
use std::time::Duration;

pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionSettings>();

        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
        });

        app.add_systems(
            Update,
            (
                setup_client.run_if(|action_state: Res<ActionStateResource>| {
                    action_state.just_happened(SimulationAction::ToggleChunkBorders)
                }),
            ),
        );

        app.add_systems(OnEnter(InGameState::Connecting), setup_client);

        app.add_plugins(message_handler::ClientMessageHandlerPlugin);
    }
}

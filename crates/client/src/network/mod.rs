pub mod local_connection;
pub mod message_handler;

pub const FIXED_TIMESTEP_HZ: f64 = 60.0;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::input::resources::ActionStateResource;
use crate::lifecycle::state::InGameState;
use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;
use local_connection::setup_client;
use shared::simulation::input::types::SimulationAction;
use std::time::Duration;

use crate::network::local_connection::check_connection_state;

pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
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

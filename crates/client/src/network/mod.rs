pub mod local_connection;
pub mod message_handler;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;
use local_connection::setup_client;
use shared::simulation::input::{resources::ActionStateResource, types::SimulationAction};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use crate::network::local_connection::check_connection_state;

pub const FIXED_TIMESTEP_HZ: f64 = 60.0;

pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        let _server_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 5000));

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

        // app.register_message() server message
        // app.add_plugins(ClientMessageHandlerPlugin);

        // app.add_systems(
        //     OnEnter(InGameState::Connecting),
        //     |mut commands: Commands| {
        //         info!("Starting client connection...");
        //         commands.connect();
        //     },
        // );
    }
}

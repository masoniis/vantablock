use crate::lifecycle::{ClientLifecycleState, InGameState, SessionTopology};
use crate::network::connection::{ConnectType, InitiateConnection, RequestSingleplayerSession};
use bevy::prelude::*;
use lightyear::{netcode::Key, prelude::client::*, prelude::*};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// Sets up a basic client.
///
/// https://cbournhonesque.github.io/lightyear/book/tutorial/build_client_server.html#client
pub fn on_initiate_connection(
    trigger: On<InitiateConnection>,
    mut commands: Commands,
    mut next_client_state: ResMut<NextState<ClientLifecycleState>>,
    mut next_in_game_state: ResMut<NextState<InGameState>>,
    mut next_session_topology: ResMut<NextState<SessionTopology>>,
) {
    let event = trigger.event();

    // handle session topology and singleplayer requests
    match event.connect_type {
        ConnectType::Singleplayer => {
            next_session_topology.set(SessionTopology::Internal);
            commands.trigger(RequestSingleplayerSession);
        }
        ConnectType::Multiplayer => {
            next_session_topology.set(SessionTopology::External);
        }
    }

    let server_addr: SocketAddr = match event.server_addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!(
                "Failed to parse server address '{}': {}. Aborting connection.",
                event.server_addr, e
            );
            return;
        }
    };

    let client_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));

    info!("Connecting client to {}", server_addr);

    // netcode auth
    let client_id = rand::random::<u64>();
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: Key::default(),
        protocol_id: 0,
    };

    // main client ent
    let client_entity = commands
        .spawn((
            LocalAddr(client_addr),
            PeerAddr(server_addr),
            Link::default(),
            ReplicationReceiver,
            // enables rollback for predicted entities
            PredictionManager::default(),
            input_timeline_config(),
            NetcodeClient::new(auth, NetcodeConfig::default())
                .expect("CRITICAL: Failed to create NetcodeClient!"),
            UdpIo::default(),
        ))
        .id();

    // trigger connection
    commands.trigger(Connect {
        entity: client_entity,
    });

    // transition to game state
    next_client_state.set(ClientLifecycleState::InGame);
    next_in_game_state.set(InGameState::Connecting);
}

/// Configures how far ahead of the server the client's input timeline runs.
///
/// At our slow tick rate, a client that settles level with the server sends every input just
/// after the server simulated its tick, and the input is dropped. We aim a full tick ahead
/// (margins are in ticks) and correct drift past half a tick, keeping inputs early but bounded.
fn input_timeline_config() -> InputTimelineConfig {
    InputTimelineConfig::default().with_sync_config(SyncConfig {
        jitter_margin: 1.0,
        error_margin: 0.5,
        ..default()
    })
}

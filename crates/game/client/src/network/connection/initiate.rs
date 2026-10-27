use crate::lifecycle::{ClientLifecycleState, InGameState, SessionTopology};
use crate::network::connection::{
    ConnectType, InitiateConnection, NetworkErrorEvent, RequestSingleplayerSession, SessionWorld,
};
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

    // parse before starting anything, so a bad address leaves no background server behind
    let server_addr: SocketAddr = match event.server_addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!(
                "Failed to parse server address '{}': {}. Aborting connection.",
                event.server_addr, e
            );
            next_client_state.set(ClientLifecycleState::Error);
            commands.trigger(NetworkErrorEvent {
                reason: format!("Invalid server address \"{}\": {e}", event.server_addr),
            });
            return;
        }
    };

    // handle session topology and singleplayer requests
    match event.connect_type {
        ConnectType::Singleplayer(world) => {
            next_session_topology.set(SessionTopology::Internal);
            commands.insert_resource(SessionWorld(world));
            commands.trigger(RequestSingleplayerSession { world });
        }
        ConnectType::Multiplayer => {
            next_session_topology.set(SessionTopology::External);
            commands.remove_resource::<SessionWorld>();
        }
    }

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

    // enables rollback for predicted entities. It is a global resource, not a component of the
    // client entity: in Bevy 0.19 a resource lives on an entity, so spawning it with the client
    // made despawning the client panic. `end_session` removes it with the connection.
    commands.insert_resource(PredictionManager::default());

    // also a global resource in lightyear 0.30: one on the client entity would be ignored
    commands.insert_resource(input_timeline_config());

    // main client ent
    let client_entity = commands
        .spawn((
            LocalAddr(client_addr),
            PeerAddr(server_addr),
            Link::default(),
            ReplicationReceiver,
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

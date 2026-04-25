use bevy::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use ::client::network::ClientNetworkPlugin;
use ::server::network::ServerNetworkPlugin;
use shared::network::SharedNetworkPlugin;
use lightyear::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::server::*;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

pub struct TestEnvironment {
    pub server_app: App,
    pub client_app: App,
    pub server_entity: Entity,
    pub client_entity: Entity,
}

impl Default for TestEnvironment {
    fn default() -> Self {
        let mut server_app = App::new();
        let mut client_app = App::new();

        // minimal bevy plugins + states
        server_app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        client_app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));

        // networking setup
        server_app.add_plugins(SharedNetworkPlugin);
        server_app.add_plugins(ServerNetworkPlugin);
        client_app.add_plugins(SharedNetworkPlugin);
        client_app.add_plugins(ClientNetworkPlugin);

        // Initialize states so that run_if(in_state(...)) systems work
        server_app.insert_state(shared::lifecycle::state::enums::AppState::Running);
        server_app.insert_state(::server::lifecycle::state::ServerState::Running);

        client_app.insert_state(shared::lifecycle::state::enums::AppState::Running);
        client_app.insert_state(::client::lifecycle::state::enums::ClientState::InGame);
        client_app.insert_state(::client::lifecycle::state::enums::InGameState::Playing);

        // 1. START UDP SERVER
        let server_entity = server_app.world_mut().run_system_once(|mut commands: Commands| {
            let server_addr = SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::LOCALHOST,
                shared::network::NETWORK_DEFAULT_PORT,
            ));

            let server_entity = commands
                .spawn((
                    NetcodeServer::new(lightyear::prelude::server::NetcodeConfig::default()),
                    LocalAddr(server_addr),
                    ServerUdpIo::default(),
                ))
                .id();

            // start listening
            commands.trigger(Start {
                entity: server_entity,
            });
            server_entity
        }).expect("Failed to run start_udp_server system");

        // 2. SETUP CLIENT
        let client_entity = client_app.world_mut().run_system_once(|mut commands: Commands| {
            let server_addr = SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::LOCALHOST,
                shared::network::NETWORK_DEFAULT_PORT,
            ));
            let client_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));

            // netcode auth
            let auth = Authentication::Manual {
                server_addr,
                client_id: 1,
                private_key: lightyear::netcode::Key::default(),
                protocol_id: 0,
            };

            // main client ent
            let client_entity = commands
                .spawn((
                    lightyear::prelude::client::Client::default(),
                    Link::default(),
                    LocalAddr(client_addr),
                    PeerAddr(server_addr),
                    NetcodeClient::new(auth, lightyear::prelude::client::NetcodeConfig::default())
                        .expect("CRITICAL: Failed to create NetcodeClient!"),
                    lightyear::prelude::UdpIo::default(),
                ))
                .id();

            // trigger connection
            commands.trigger(Connect {
                entity: client_entity,
            });
            client_entity
        }).expect("Failed to run setup_client system");

        Self {
            server_app,
            client_app,
            server_entity,
            client_entity,
        }
    }
}

impl TestEnvironment {
    /// Helper to tick both apps so packets actually process
    pub fn step(&mut self) {
        use std::time::Duration;
        let delta = Duration::from_secs_f32(1.0 / 60.0);

        // manually advance time so lightyear can progress its state machine
        for app in [&mut self.server_app, &mut self.client_app] {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(delta);
        }

        self.server_app.update();
        self.client_app.update();
    }

    /// Step multiple times to allow for handshakes and message propagation
    pub fn step_frames(&mut self, frames: usize) {
        for _ in 0..frames {
            self.step();
        }
    }
}


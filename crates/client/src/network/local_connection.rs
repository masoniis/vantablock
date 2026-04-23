use crate::network::resources::ConnectionSettings;
use bevy::prelude::*;
use lightyear::netcode::Key;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// Sets up a basic client.
///
/// https://cbournhonesque.github.io/lightyear/book/tutorial/build_client_server.html#client
pub fn setup_client(mut commands: Commands, settings: Res<ConnectionSettings>) {
    let server_addr: SocketAddr = settings.server_addr.parse().unwrap_or_else(|_| {
        error!("Failed to parse server address. Falling back to 127.0.0.1:5000");
        "127.0.0.1:5000".parse().unwrap()
    });

    let client_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));

    info!("Connecting client to {}", server_addr);

    // netcode auth
    let auth = Authentication::Manual {
        server_addr,
        client_id: 1,
        private_key: Key::default(),
        protocol_id: 0, // must match server protocol id
    };

    // main client ent
    let client_entity = commands
        .spawn((
            Client::default(),
            Link::default(),
            LocalAddr(client_addr),
            PeerAddr(server_addr),
            NetcodeClient::new(auth, NetcodeConfig::default())
                .expect("CRITICAL: Failed to create NetcodeClient!"),
            UdpIo::default(),
        ))
        .id();

    // trigger connection
    commands.trigger(Connect {
        entity: client_entity,
    });
}

use bevy::prelude::*;
use lightyear::{prelude::server::*, prelude::*};
use shared::network::DEFAULT_SERVER_PORT;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tracing::info;

/// Starts a udp server listening on an available local addr
///
/// https://cbournhonesque.github.io/lightyear/book/tutorial/build_client_server.html#server
pub fn start_udp_server(mut commands: Commands) {
    let server_addr = SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        DEFAULT_SERVER_PORT,
    ));

    info!("Starting server listening on {}...", server_addr);

    let server_entity = commands
        .spawn((
            Server::default(),
            Link::default(),
            NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(server_addr),
            ServerUdpIo::default(),
        ))
        .id();

    // start listening
    commands.trigger(Start {
        entity: server_entity,
    });
}

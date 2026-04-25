use crate::{prelude::*, simulation::chunk_loading::ClientChunkTracker};
use bevy::{
    ecs::lifecycle::Add,
    ecs::{observer::On, system::Commands},
    prelude::{Component, Entity, Transform},
};
use lightyear::netcode::NetcodeServer;
use lightyear::prelude::server::{NetcodeConfig, ServerUdpIo, Start};
use lightyear::prelude::{Connected, LocalAddr, MessageSender};
use shared::network::{NETWORK_DEFAULT_PORT, ServerMessage};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

#[derive(Component)]
pub struct ClientConnection {
    pub client_entity: Entity,
}

/// Starts a udp server listening on an available local addr
///
/// https://cbournhonesque.github.io/lightyear/book/tutorial/build_client_server.html#server
pub fn start_udp_server(mut commands: Commands) {
    let server_addr = SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        NETWORK_DEFAULT_PORT,
    ));

    info!("Starting server listening on {}...", server_addr);

    let server_entity = commands
        .spawn((
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

pub fn handle_connections(trigger: On<Add, Connected>, mut commands: Commands) {
    let client_entity = trigger.entity;

    info!("Client connected with entity: {:?}", client_entity);

    let spawn_pos = Vec3::new(0.0, 60.0, 0.0);

    // ensure client entity has MessageSender
    commands
        .entity(client_entity)
        .insert(MessageSender::<ServerMessage>::default());

    let player_entity = commands
        .spawn((
            ClientConnection { client_entity },
            ClientChunkTracker::default(),
            Transform::from_translation(spawn_pos),
        ))
        .id();

    info!("Player ent spawned {:?}", player_entity);
}

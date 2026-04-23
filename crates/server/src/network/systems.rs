use crate::prelude::*;
use crate::simulation::chunk_loading::ClientChunkTracker;
use bevy::ecs::{observer::On, system::Commands};
use bevy::prelude::{Component, Entity, Transform};
use lightyear::prelude::{Connect, Link, MessageSender, Server};
use shared::network::NETWORK_DEFAULT_PORT;
use shared::network::protocol::server::ServerMessage;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

pub fn start_server(mut commands: Commands) {
    let server_addr = SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        NETWORK_DEFAULT_PORT,
    ));
    info!("Starting server listening on {}...", server_addr);

    let server_entity = commands.spawn((Server::default(), Link::default())).id();

    // start listening
    commands.trigger(lightyear::prelude::server::Start {
        entity: server_entity,
    });
}

#[derive(Component)]
pub struct ClientConnection {
    pub client_entity: Entity,
}

#[derive(Component)]
pub struct SentWelcome;

pub fn handle_connections(trigger: On<Connect>, mut commands: Commands) {
    let client_entity = trigger.entity;
    info!("Client connected with entity: {:?}", client_entity);

    // spawn a player entity for the client
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

use crate::prelude::*;
use bevy::ecs::{observer::On, system::Commands};
use lightyear::prelude::{Connect, Link, Server};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

pub fn start_server(mut commands: Commands) {
    let server_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5000));
    info!("Starting server listening on {}...", server_addr);

    let server_entity = commands.spawn((Server::default(), Link::default())).id();

    // start listening
    commands.trigger(lightyear::prelude::server::Start {
        entity: server_entity,
    });
}

pub fn handle_connections(trigger: On<Connect>, mut commands: Commands) {
    let client_entity = trigger.entity;
    info!("Client connected with entity: {:?}", client_entity);

    // spawn a player entity for the client
    let player_entity = commands.spawn_empty().id();
    info!("Player ent spawned {:?}", player_entity);
}

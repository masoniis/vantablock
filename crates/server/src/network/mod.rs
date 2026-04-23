pub const FIXED_TIMESTEP_HZ: f64 = 60.0;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::prelude::*;
use bevy::{
    app::Startup,
    ecs::{observer::On, system::Commands},
    prelude::Plugin,
};
use lightyear::prelude::{server::ServerPlugins, Connect, Link, Server};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
        });

        app.add_systems(Startup, start_server)
            .add_observer(handle_connections);
    }
}

fn start_server(mut commands: Commands) {
    let server_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5000));
    info!("Starting server listening on {}...", server_addr);

    let server_entity = commands.spawn((Server::default(), Link::default())).id();

    // start listening
    commands.trigger(lightyear::prelude::server::Start {
        entity: server_entity,
    });
}

fn handle_connections(trigger: On<Connect>, mut commands: Commands) {
    let client_entity = trigger.entity;
    info!("Client connected with entity: {:?}", client_entity);

    // spawn a player entity for the client
    let player_entity = commands.spawn_empty().id();
    info!("Player ent spawned {:?}", player_entity);
}

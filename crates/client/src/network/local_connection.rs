use bevy::prelude::*;
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};

pub fn setup_client(mut commands: Commands) {
    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);

    let client_entity = commands
        .spawn((
            Client::default(),
            Link::default(),
            PeerAddr(server_addr),
            MessageManager::default(),
            ReplicationSender::default(),
            ReplicationReceiver::default(),
        ))
        .id();

    info!("Trigging client spawn/connect",);
    commands.trigger(Connect {
        entity: client_entity,
    });
}

pub fn check_connection_state(connected_clients: Query<Entity, (With<Client>, With<Connected>)>) {
    for entity in connected_clients.iter() {
        info!(
            "Client {:?} is successfully connected to the server!",
            entity
        );
    }
}

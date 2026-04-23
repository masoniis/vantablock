use crate::network::resources::ConnectionSettings;
use bevy::prelude::*;
use lightyear::prelude::*;
use std::net::SocketAddr;

pub fn setup_client(mut commands: Commands, settings: Res<ConnectionSettings>) {
    let server_addr: SocketAddr = settings.server_addr.parse().unwrap_or_else(|_| {
        error!(
            "Failed to parse server address \"{}\". Falling back to 127.0.0.1:5000",
            settings.server_addr
        );
        "127.0.0.1:5000".parse().unwrap()
    });

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

    info!("Trigging client spawn/connect to {}", server_addr);
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

use bevy::prelude::*;
use lightyear::crossbeam::CrossbeamIo;
use lightyear::prelude::client::{Client, Connect};
use lightyear::prelude::server::{Server, Start};
use lightyear::prelude::*;

/// Links a Server App and a Client App via Crossbeam.
pub fn setup_crossbeam_connection(server_app: &mut App, client_app: &mut App) -> (Entity, Entity) {
    let (server_io, client_io) = CrossbeamIo::new_pair();

    // setup server
    let server_entity = server_app
        .world_mut()
        .spawn((
            Server::default(),
            Link::default(),
            server_io, // server crossbeam io
        ))
        .id();

    server_app.world_mut().trigger(Start {
        entity: server_entity,
    });

    // setup client
    let client_entity = client_app
        .world_mut()
        .spawn((
            Client::default(),
            Link::default(),
            client_io, // client crossbeam io
        ))
        .id();

    client_app.world_mut().trigger(Connect {
        entity: client_entity,
    });

    (server_entity, client_entity)
}

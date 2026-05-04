use crate::{network::types::ClientConnection, world::chunk_loading::ClientChunkTracker};
use bevy::prelude::*;
use lightyear::{prelude::server::*, prelude::*};
use shared::network::{ChatAndSystem, ServerMessage};
use std::time::Duration;
use tracing::{error, info};

pub fn on_client_connect(
    trigger: On<Add, Connected>,
    mut commands: Commands,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Server>>,
    client_ids: Query<&RemoteId>,
) {
    let client_entity = trigger.entity;

    info!("Client connected with entity: {:?}", client_entity);

    let spawn_pos = Vec3::new(0.0, 120.0, 60.0);

    // ensure client entity has MessageSender and ReplicationSender
    commands
        .entity(client_entity)
        .insert(ReplicationSender::new(
            Duration::from_millis(100),
            SendUpdatesMode::SinceLastAck,
            false,
        ))
        .insert(MessageSender::<ServerMessage>::default());

    let player_entity = commands
        .spawn((
            shared::player::components::NetworkPlayer,
            shared::player::components::PlayerLook::default(),
            shared::player::components::LogicalPosition(spawn_pos),
            ClientConnection { client_entity },
            ClientChunkTracker::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();

    info!("Player ent spawned {:?}", player_entity);

    // send welcome message
    if let Some(server) = server
        && let Ok(remote_id) = client_ids.get(client_entity)
        && let Err(e) = sender.send::<_, ChatAndSystem>(
            &ServerMessage::Welcome {
                entity_id: player_entity,
                spawn_pos,
            },
            server.into_inner(),
            &NetworkTarget::Only(vec![**remote_id]),
        )
    {
        error!(
            "Failed to send Welcome message to client {:?}: {:?}",
            client_entity, e
        );
    }
}

pub fn on_client_disconnect(
    trigger: On<Remove, Connected>,
    mut commands: Commands,
    player_query: Query<(Entity, &ClientConnection)>,
) {
    let client_entity = trigger.entity;
    info!("Client disconnected: {:?}", client_entity);

    for (player_entity, connection) in player_query.iter() {
        if connection.client_entity == client_entity {
            info!("Cleaning up player entity {:?}", player_entity);
            commands.entity(player_entity).despawn();
        }
    }
}

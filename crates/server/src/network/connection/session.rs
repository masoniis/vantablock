use crate::{network::types::ClientConnection, world::chunk_loading::ClientChunkTracker};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::{
    network::ServerMessage,
    player::components::{LogicalPosition, NetworkPlayer, PlayerLook, PlayerOwner},
};
use std::time::Duration;
use tracing::{error, info};

pub fn on_client_connect(
    trigger: On<Add, Connected>,
    mut commands: Commands,
    client_ids: Query<&RemoteId>,
) {
    let client_entity = trigger.entity;

    info!("Client connected with entity: {:?}", client_entity);

    let spawn_pos = Vec3::new(0.0, 120.0, 60.0);

    let Ok(remote_id) = client_ids.get(client_entity) else {
        error!(
            "Could not find RemoteId for client entity {:?}",
            client_entity
        );
        return;
    };
    let client_id = **remote_id;

    // ensure client entity has MessageSender and ReplicationSender
    commands
        .entity(client_entity)
        .insert(ReplicationSender::new(
            Duration::from_millis(100),
            SendUpdatesMode::SinceLastAck,
            false,
        ))
        .insert(MessageSender::<ServerMessage>::default());

    let client_player_entity = commands
        .spawn((
            NetworkPlayer,
            PlayerOwner(client_id),
            PlayerLook::default(),
            LogicalPosition(spawn_pos),
            ClientConnection { client_entity },
            ClientChunkTracker::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();

    info!("Player ent spawned {:?}", client_player_entity);
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

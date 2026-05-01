use crate::world::chunk_loading::ClientChunkTracker;
use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::network::{ChatAndSystem, DEFAULT_SERVER_PORT, ServerMessage};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tracing::{error, info};

#[derive(Component)]
pub struct ClientConnection {
    pub client_entity: Entity,
}

#[derive(Resource)]
pub struct MessageTimer(pub Timer);

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

pub fn handle_connections(trigger: On<Add, Connected>, mut commands: Commands) {
    let client_entity = trigger.entity;

    info!("Client connected with entity: {:?}", client_entity);

    let spawn_pos = Vec3::new(0.0, 120.0, 60.0);

    // ensure client entity has MessageSender and ReplicationSender
    commands
        .entity(client_entity)
        .insert(ReplicationSender::new(
            std::time::Duration::from_millis(100),
            SendUpdatesMode::SinceLastAck,
            false,
        ))
        .insert(MessageSender::<ServerMessage>::default());

    let player_entity = commands
        .spawn((
            shared::player::components::Player,
            shared::player::components::PlayerLook::default(),
            shared::player::components::LogicalPosition(spawn_pos),
            ClientConnection { client_entity },
            ClientChunkTracker::default(),
            Transform::from_translation(spawn_pos),
        ))
        .id();

    info!("Player ent spawned {:?}", player_entity);
}

pub fn handle_disconnections(
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

pub fn send_sync_time(
    mut timer: ResMut<MessageTimer>,
    time: Res<Time>,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Server>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished()
        && let Some(server) = server
    {
        let message = ServerMessage::SyncTime {
            game_time: time.elapsed_secs(),
            tick: 0, // placeholder
        };
        info!("Sending periodic SyncTime message: {:?}", message);
        if let Err(e) =
            sender.send::<_, ChatAndSystem>(&message, server.into_inner(), &NetworkTarget::All)
        {
            error!("Failed to send SyncTime message: {:?}", e);
        }
    }
}

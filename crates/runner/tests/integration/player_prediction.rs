use bevy::prelude::*;
use client::{
    HeadlessClientPlugins,
    network::connection::{ConnectType, InitiateConnection},
    player::{LocalPlayer, LocalPlayerLook},
};
use lightyear::prelude::Predicted;
use server::{DefaultServerPlugins, lifecycle::state::ServerState};
use shared::{
    lifecycle::state::AppState,
    network::DEFAULT_LOCAL_SERVER_ADDR,
    player::components::{LogicalPosition, NetworkPlayer, PlayerLook},
    world::{ObjectPlacement, WorldObject},
};
use std::time::{Duration, Instant};

const TARGET_YAW: f32 = 0.5;

/// Steps both apps in real time until `condition` holds, panicking on timeout.
fn step_until(
    server: &mut App,
    client: &mut App,
    timeout: Duration,
    what: &str,
    mut condition: impl FnMut(&mut App, &mut App) -> bool,
) {
    let start = Instant::now();
    while !condition(server, client) {
        assert!(start.elapsed() < timeout, "timed out waiting for: {what}");
        server.update();
        client.update();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn server_player(server: &mut App) -> Option<(Vec3, PlayerLook)> {
    let mut query = server
        .world_mut()
        .query_filtered::<(&LogicalPosition, &PlayerLook), With<NetworkPlayer>>();
    query
        .iter(server.world())
        .next()
        .map(|(pos, look)| (pos.0, *look))
}

fn client_local_player(client: &mut App) -> Option<(Entity, Vec3, PlayerLook)> {
    let mut query = client.world_mut().query_filtered::<
        (Entity, &LogicalPosition, &PlayerLook),
        (With<LocalPlayer>, With<Predicted>),
    >();
    query
        .iter(client.world())
        .next()
        .map(|(entity, pos, look)| (entity, pos.0, *look))
}

fn world_objects(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<(), (With<WorldObject>, With<ObjectPlacement>)>();
    query.iter(app.world()).count()
}

/// Connects a headless client to a real server over local UDP, then verifies that the
/// server's world objects replicate, that the client's movement and look inputs reach the
/// server, and that the client's predicted state agrees with the server's authoritative state.
#[test]
fn world_replicates_and_player_inputs_are_predicted() {
    utils::attach_logger();

    let mut server = App::new();
    server.add_plugins(DefaultServerPlugins);
    let mut client = App::new();
    client.add_plugins(HeadlessClientPlugins);

    // `App::run` normally does this, but we drive the apps by hand. Lightyear builds its
    // replication systems in `Plugin::finish`, so skipping this silently disables replication.
    for app in [&mut server, &mut client] {
        app.finish();
        app.cleanup();
    }

    // boot server and client to their idle states
    step_until(
        &mut server,
        &mut client,
        Duration::from_secs(60),
        "server running and client loaded",
        |server, client| {
            let server_ready = server
                .world()
                .get_resource::<State<ServerState>>()
                .is_some_and(|s| *s.get() == ServerState::Running);
            let client_ready = client
                .world()
                .get_resource::<State<AppState>>()
                .is_some_and(|s| *s.get() == AppState::Running);
            server_ready && client_ready
        },
    );

    client.world_mut().trigger(InitiateConnection {
        connect_type: ConnectType::Multiplayer,
        server_addr: DEFAULT_LOCAL_SERVER_ADDR.to_string(),
    });

    // the server marks the owner's player as predicted, which the client dresses as local
    step_until(
        &mut server,
        &mut client,
        Duration::from_secs(20),
        "predicted local player on client",
        |_, client| client_local_player(client).is_some(),
    );

    // every server-owned world object replicates to the client with its placement
    let server_objects = world_objects(&mut server);
    assert!(
        server_objects > 0,
        "the server should populate its showcase scene"
    );
    step_until(
        &mut server,
        &mut client,
        Duration::from_secs(20),
        "world objects replicated to the client",
        |server, client| world_objects(client) == world_objects(server),
    );

    let (local_entity, _, _) = client_local_player(&mut client).unwrap();
    let (spawn_pos, _) = server_player(&mut server).expect("server should have a player");

    // turn and hold forward
    client
        .world_mut()
        .get_mut::<LocalPlayerLook>(local_entity)
        .unwrap()
        .yaw = TARGET_YAW;
    client
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);

    // the server must authoritatively apply both the look and the movement inputs
    step_until(
        &mut server,
        &mut client,
        Duration::from_secs(20),
        "server applying client look and movement",
        |server, _| {
            server_player(server)
                .is_some_and(|(pos, look)| look.yaw == TARGET_YAW && pos.distance(spawn_pos) > 5.0)
        },
    );

    // release and let the server's authoritative state settle on the client
    client
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyW);
    let settle_start = Instant::now();
    while settle_start.elapsed() < Duration::from_secs(2) {
        server.update();
        client.update();
        std::thread::sleep(Duration::from_millis(5));
    }

    let (server_pos, server_look) = server_player(&mut server).unwrap();
    let (_, client_pos, client_look) = client_local_player(&mut client).unwrap();

    // the client's look must not have been snapped back by replication
    assert_eq!(client_look.yaw, TARGET_YAW);
    assert_eq!(server_look.yaw, TARGET_YAW);
    assert!(
        client_pos.distance(server_pos) < 0.01,
        "client prediction {client_pos} diverged from server {server_pos}"
    );
}

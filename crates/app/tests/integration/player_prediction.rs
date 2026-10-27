use bevy::prelude::*;
use client::{
    HeadlessClientPlugins,
    input::{local_actions::ClientAction, systems::toggle_chunk_borders::ChunkBoundsToggle},
    lifecycle::ClientLifecycleState,
    network::connection::{ConnectType, InitiateConnection},
    player::{LocalPlayer, LocalPlayerLook},
    showcase::{ApplyShowcase, SHOWCASES},
    world::chunk::ClientChunkMap,
};
use common::{
    lifecycle::state::AppState,
    network::DEFAULT_LOCAL_SERVER_ADDR,
    player::components::{LogicalPosition, NetworkPlayer, PlayerLook},
};
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::{Client, Predicted, PredictionManager};
use server::{DefaultServerPlugins, lifecycle::state::ServerState};
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

/// Connects a headless client to a real server over local UDP, then verifies that the
/// client's movement and look inputs reach the server, that the client's predicted state agrees
/// with the server's authoritative state, and that the session can be left and rejoined.
#[test]
fn local_player_inputs_are_predicted_and_authoritative() {
    platform::attach_logger();

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

    // client-only binds read a single `ActionState<ClientAction>`, so joining must not add one
    let mut action_states = client.world_mut().query::<&ActionState<ClientAction>>();
    assert_eq!(action_states.iter(client.world()).count(), 1);
    client
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyB);
    step_until(
        &mut server,
        &mut client,
        Duration::from_secs(5),
        "chunk borders toggled in game",
        |_, client| client.world().resource::<ChunkBoundsToggle>().enabled,
    );
    client
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);

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

    // a showcase moves the player on the server, and prediction follows it
    client.world_mut().trigger(ApplyShowcase(0));
    let target = SHOWCASES[0].position;
    step_until(
        &mut server,
        &mut client,
        Duration::from_secs(10),
        "showcase teleport applied on server and client",
        |server, client| {
            server_player(server).is_some_and(|(pos, _)| pos == target)
                && client_local_player(client)
                    .is_some_and(|(_, pos, _)| pos.distance(target) < 0.01)
        },
    );

    // leave the session the way the pause menu's Disconnect does and join again, a few times
    // over: teardown must not panic or leave a client entity, prediction resource or chunk
    // behind, and each new session must predict its new local player and stream chunks again
    for _ in 0..3 {
        assert!(
            tracked_chunk_count(&client) > 0,
            "client should be streaming chunks before leaving"
        );

        client
            .world_mut()
            .resource_mut::<NextState<ClientLifecycleState>>()
            .set(ClientLifecycleState::MainMenu);
        step_until(
            &mut server,
            &mut client,
            Duration::from_secs(5),
            "session torn down",
            |_, client| {
                let mut clients = client.world_mut().query_filtered::<(), With<Client>>();
                clients.iter(client.world()).next().is_none()
                    && !client.world().contains_resource::<PredictionManager>()
                    && client_local_player(client).is_none()
                    && tracked_chunk_count(client) == 0
            },
        );

        client.world_mut().trigger(InitiateConnection {
            connect_type: ConnectType::Multiplayer,
            server_addr: DEFAULT_LOCAL_SERVER_ADDR.to_string(),
        });
        step_until(
            &mut server,
            &mut client,
            Duration::from_secs(20),
            "predicted local player and chunks after rejoining",
            |_, client| client_local_player(client).is_some() && tracked_chunk_count(client) > 0,
        );
    }
}

fn tracked_chunk_count(client: &App) -> usize {
    client
        .world()
        .resource::<ClientChunkMap>()
        .chunk_states
        .len()
}

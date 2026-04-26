//! The server side of the example.
//! It is possible (and recommended) to run the server in headless mode (without any rendering plugins).
//!
//! The server will:
//! - spawn a new player entity for each client that connects
//! - read inputs from the clients and move the player entities accordingly
//!
//! Lightyear will handle the replication of entities automatically if you add a `Replicate` component to them.
use crate::shared::*;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

pub struct ExampleServerPlugin;

#[derive(Resource)]
struct MessageTimer(Timer);

impl Plugin for ExampleServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MessageTimer(Timer::from_seconds(5.0, TimerMode::Repeating)));
        app.add_systems(Startup, startup);
        app.add_systems(Update, send_message);
        app.add_observer(handle_new_client);
    }
}

fn send_message(
    mut timer: ResMut<MessageTimer>,
    time: Res<Time>,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Server>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        if let Some(server) = server {
            let message = Message1(42);
            info!("Sending message: {:?}", message);
            let _ = sender.send::<_, Channel1>(&message, server.into_inner(), &NetworkTarget::All);
        }
    }
}

/// Whenever a new client connects to the server, a new entity will get spawned with
/// the `Connected` component, which represents the connection between the server and that specific client.
///
/// You can add more components to customize how this connection, for example by adding a
/// `ReplicationSender` (so that the server can send replication updates to that client)
/// or a `MessageSender`.
fn handle_new_client(trigger: On<Add, Connected>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(ReplicationSender::new(
            SERVER_REPLICATION_INTERVAL,
            SendUpdatesMode::SinceLastAck,
            false,
        ))
        .insert(MessageSender::<Message1>::default());
}

/// Start the server
fn startup(mut commands: Commands) -> Result {
    let server = commands
        .spawn((
            NetcodeServer::new(server::NetcodeConfig::default()),
            LocalAddr(SERVER_ADDR),
            ServerUdpIo::default(),
        ))
        .id();
    commands.trigger(Start { entity: server });
    Ok(())
}

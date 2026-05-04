use bevy::prelude::*;
use server::{DefaultServerPlugins, lifecycle::state::ServerState};
use std::{
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};
use tracing::info;

#[cfg(feature = "client")]
use client::network::connection::RequestSingleplayerSession;

pub struct OrchestratorPlugin;

impl Plugin for OrchestratorPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "client")]
        app.add_observer(handle_request_singleplayer);
    }
}

/// Resource inserted into the Main (Client) App to watch for the server
#[derive(Resource)]
pub struct LocalServerFlag(pub Arc<AtomicBool>);

/// Resource inserted into the Background Server App so it knows how to notify the client
#[derive(Resource)]
struct ServerReadyNotifier(Arc<AtomicBool>);

#[cfg(feature = "client")]
fn handle_request_singleplayer(_trigger: On<RequestSingleplayerSession>, mut commands: Commands) {
    info!("Orchestrator: Received RequestSingleplayerSession. Starting background server...");

    // create the thread-safe flag
    let server_is_ready = Arc::new(AtomicBool::new(false));
    let flag_for_server = Arc::clone(&server_is_ready);

    // insert the flag into the Main App (so the Client can poll it)
    commands.insert_resource(LocalServerFlag(server_is_ready));

    // spawn the background server
    thread::spawn(move || {
        utils::set_runtime_context_server();

        let mut app = App::new();
        app.add_plugins(DefaultServerPlugins);

        // pass the server's half of the flag into its ECS
        app.insert_resource(ServerReadyNotifier(flag_for_server));

        // tell the server to flip the flag the exact moment it enters NetworkingMode::Active
        app.add_systems(OnEnter(ServerState::Running), notify_client_ready);

        info!("Background Server: Booting...");

        app.run();
    });
}

/// System that runs exactly once on the background thread when the port is successfully bound
fn notify_client_ready(notifier: Option<Res<ServerReadyNotifier>>) {
    if let Some(flag) = notifier {
        info!("Background Server: Port is open and active! Notifying orchestrator...");
        // flip the switch!
        flag.0.store(true, Ordering::Relaxed);
    }
}

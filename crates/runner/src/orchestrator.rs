use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use bevy::prelude::*;
use server::lifecycle::state::ServerState;
use server::DefaultServerPlugins;
use shared::events::RequestSingleplayerSession;
use tracing::info;

pub struct OrchestratorPlugin;

impl Plugin for OrchestratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_request_singleplayer);
    }
}

/// Resource inserted into the Main (Client) App to watch for the server
#[derive(Resource)]
pub struct LocalServerFlag(pub Arc<AtomicBool>);

/// Resource inserted into the Background Server App so it knows how to notify the client
#[derive(Resource)]
struct ServerReadyNotifier(Arc<AtomicBool>);

fn handle_request_singleplayer(
    mut commands: Commands,
    mut ev_request_session: MessageReader<RequestSingleplayerSession>,
) {
    for _ in ev_request_session.read() {
        info!("Orchestrator: Received RequestSingleplayerSession. Starting background server...");

        // create the thread-safe flag
        let server_is_ready = Arc::new(AtomicBool::new(false));
        let flag_for_server = Arc::clone(&server_is_ready);

        // insert the flag into the Main App (so the Client can poll it)
        commands.insert_resource(LocalServerFlag(server_is_ready));

        // Spawn the background server
        thread::spawn(move || {
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
}

/// System that runs exactly once on the background thread when the port is successfully bound
fn notify_client_ready(notifier: Option<Res<ServerReadyNotifier>>) {
    if let Some(flag) = notifier {
        info!("Background Server: Port is open and active! Notifying orchestrator...");
        // flip the switch!
        flag.0.store(true, Ordering::Relaxed);
    }
}

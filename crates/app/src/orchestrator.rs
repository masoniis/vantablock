use bevy::prelude::*;
use server::{DefaultServerPlugins, lifecycle::state::ServerState};
use std::{
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, JoinHandle},
};
use tracing::info;

use client::{lifecycle::ClientLifecycleState, network::connection::RequestSingleplayerSession};

pub struct OrchestratorPlugin;

impl Plugin for OrchestratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_request_singleplayer);
        app.add_systems(OnExit(ClientLifecycleState::InGame), stop_local_server);
    }
}

/// Resource inserted into the Main (Client) App to track the background server
#[derive(Resource)]
pub struct LocalServer {
    /// Flipped by the server once its port is bound
    pub ready: Arc<AtomicBool>,
    /// Flipped by the client to ask the server to exit
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// Resource inserted into the Background Server App so it knows how to talk to the client
#[derive(Resource)]
struct LocalServerSignals {
    ready: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

fn handle_request_singleplayer(
    _trigger: On<RequestSingleplayerSession>,
    mut commands: Commands,
    previous: Option<ResMut<LocalServer>>,
) {
    info!("Orchestrator: Received RequestSingleplayerSession. Starting background server...");

    // a previous session's server may still be shutting down and holding the port
    let previous_thread = previous.and_then(|mut server| {
        server.shutdown.store(true, Ordering::Relaxed);
        server.thread.take()
    });

    let ready = Arc::new(AtomicBool::new(false));
    let shutdown = Arc::new(AtomicBool::new(false));
    let signals = LocalServerSignals {
        ready: Arc::clone(&ready),
        shutdown: Arc::clone(&shutdown),
    };

    // spawn the background server
    let thread = thread::spawn(move || {
        platform::set_runtime_context_server();

        if let Some(previous_thread) = previous_thread {
            info!("Background Server: Waiting for previous server to exit...");
            let _ = previous_thread.join();
        }

        let mut app = App::new();
        app.add_plugins(DefaultServerPlugins);

        // pass the server's half of the signals into its ECS
        app.insert_resource(signals);

        // tell the server to flip the flag the exact moment it enters NetworkingMode::Active
        app.add_systems(OnEnter(ServerState::Running), notify_client_ready);
        app.add_systems(Update, exit_on_shutdown_request);

        info!("Background Server: Booting...");

        app.run();

        info!("Background Server: Exited.");
    });

    commands.insert_resource(LocalServer {
        ready,
        shutdown,
        thread: Some(thread),
    });
}

/// Asks the background server to exit once the client leaves the game
fn stop_local_server(server: Option<Res<LocalServer>>) {
    if let Some(server) = server {
        info!("Orchestrator: Session ended. Stopping background server...");
        server.shutdown.store(true, Ordering::Relaxed);
    }
}

/// System that runs exactly once on the background thread when the port is successfully bound
fn notify_client_ready(signals: Res<LocalServerSignals>) {
    info!("Background Server: Port is open and active! Notifying orchestrator...");
    signals.ready.store(true, Ordering::Relaxed);
}

/// Exits the background server app when the client requests it
fn exit_on_shutdown_request(signals: Res<LocalServerSignals>, mut exit: MessageWriter<AppExit>) {
    if signals.shutdown.load(Ordering::Relaxed) {
        exit.write(AppExit::Success);
    }
}

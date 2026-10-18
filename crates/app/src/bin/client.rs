#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app::orchestrator::OrchestratorPlugin;
use bevy::{
    app::{App, PostUpdate},
    prelude::info,
};
use client::{lifecycle::scheduling::RenderPrepSet, prelude::*};

/// The main entrypoint for the entire game.
#[instrument(skip_all, fields(name = "main"))]
fn main() {
    platform::set_runtime_context_client();
    platform::attach_logger();

    info!("Building client app...");

    let mut app = App::new();

    // config of default plugins
    app.add_plugins(client::DefaultClientPlugins);

    // the orchestrator will handle spinning up a background server if requested
    app.add_plugins(OrchestratorPlugin);

    // set ordering
    app.configure_sets(PostUpdate, RenderPrepSet);

    info!("App built! Running the event loop.");

    app.run();

    info!("App exited safely!");
}

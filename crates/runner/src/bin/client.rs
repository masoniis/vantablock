#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    app::{App, PostUpdate},
    prelude::info,
};
use client::{lifecycle::scheduling::RenderPrepSet, prelude::*};
use vantablock_runner::orchestrator::OrchestratorPlugin;

/// The main entrypoint for the entire game.
#[instrument(skip_all, fields(name = "main"))]
fn main() {
    utils::set_runtime_context_client();
    utils::attach_logger();

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

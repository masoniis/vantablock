#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    app::{App, PostUpdate},
    prelude::info,
};
use client::{lifecycle::scheduling::RenderPrepSet, prelude::*};
use utils::attach_logger;

/// The main entrypoint for the entire game.
#[instrument(skip_all, fields(name = "main"))]
fn main() {
    attach_logger();

    info!("Building client app...");

    let mut app = App::new();

    // config of default plugins
    //
    // the client can act as both a client and a server so it gets
    // the server core as well
    app.add_plugins(client::DefaultClientPlugins);
    app.add_plugins(server::ServerCoreLogicPlugins);

    // set ordering
    app.configure_sets(PostUpdate, RenderPrepSet);

    info!("App built! Running the event loop.");

    app.run();

    info!("App exited safely!");
}

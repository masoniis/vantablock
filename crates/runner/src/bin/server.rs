use bevy::prelude::App;
use server::DefaultServerPlugins;
use tracing::info;

/// This is the entrypoint for a dedicated server. The server logic is also used
/// for a client running singleplayer, though not through the main entrypoint.
fn main() {
    // initialize logging
    utils::attach_logger();

    info!("Building server app...");

    // setup server app
    let mut app = App::new();

    // server app only has server plugins, running headless with no client
    app.add_plugins(DefaultServerPlugins);

    info!("App built! Running the event loop.");

    app.run();

    info!("App exited safely!");
}

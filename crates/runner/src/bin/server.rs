use bevy::{
    MinimalPlugins,
    app::ScheduleRunnerPlugin,
    asset::AssetPlugin,
    prelude::{App, PluginGroup, default},
    state::app::StatesPlugin,
};
use server::ServerPlugins;
use shared::SharedPlugins;
use std::time::Duration;
use tracing::info;

/// This is the entrypoint for a dedicated server. The server logic is also used
/// for a client running singleplayer, though not through the main entrypoint.
fn main() {
    // initialize logging
    utils::attach_logger();

    // setup headless bevy app
    let mut app = App::new();

    info!("Building server app...");

    // resolve platform paths for initial plugin configuration
    let persistent_paths = utils::PersistentPaths::resolve();

    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        ))),
    );

    app.add_plugins(StatesPlugin);
    app.add_plugins(AssetPlugin {
        file_path: persistent_paths.assets_dir.to_string_lossy().to_string(),
        ..default()
    });

    // add server-side and shared plugins
    app.add_plugins(ServerPlugins);
    app.add_plugins(SharedPlugins);

    info!("App built! Running the server");

    app.run();
}

use bevy::{
    app::ScheduleRunnerPlugin,
    asset::AssetPlugin,
    prelude::{App, DefaultPlugins, PluginGroup, default, info},
};
use server::ServerPlugins;
use shared::SharedPlugins;
use std::time::Duration;

/// This is the entrypoint for a dedicated server. The server logic is also used
/// for a client running singleplayer, though not through the main entrypoint.
fn main() {
    // setup headless bevy app
    let mut app = App::new();

    // resolve platform paths for initial plugin configuration
    let persistent_paths = utils::PersistentPaths::resolve();

    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        file_path: persistent_paths.assets_dir.to_string_lossy().to_string(),
        ..default()
    }));

    app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 60.0,
    )));

    // add server-side and shared plugins (handles resource insertion via plugins)
    app.add_plugins(ServerPlugins);
    app.add_plugins(SharedPlugins);

    info!("Server started successfully!");
    app.run();
}

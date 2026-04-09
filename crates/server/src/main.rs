use bevy::{
    MinimalPlugins,
    app::{App, ScheduleRunnerPlugin},
    asset::AssetPlugin,
    prelude::{PluginGroup, default, info},
};
use server::ServerPlugins;
use shared::load::LoadingTracker;
use std::time::Duration;
use utils::PersistentPaths;

fn main() {
    // setup headless bevy app
    let mut app = App::new();

    // Resolve platform paths and initialize application paths
    let persistent_paths = PersistentPaths::resolve();

    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        ))),
    );

    // AssetServer is required for registry logic to function in a standardized way.
    app.add_plugins(AssetPlugin {
        file_path: "assets".to_string(),
        ..default()
    });

    // load config & loading trackers into main world
    app.insert_resource(persistent_paths);
    app.insert_resource(LoadingTracker::default());

    // add server-side and shared plugins
    app.add_plugins(ServerPlugins);

    info!("Server started successfully!");
    app.run();
}

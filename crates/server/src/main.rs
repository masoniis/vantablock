use bevy::{
    MinimalPlugins,
    app::{App, ScheduleRunnerPlugin},
    asset::AssetPlugin,
    prelude::{PluginGroup, default, info},
};
use shared::{
    load::LoadingTracker,
    simulation::{
        asset::AssetPlugin as SimulationAssetPlugin, biome::BiomePlugin, block::BlockPlugin,
        chunk::ChunkLoadingPlugin, terrain::TerrainGenerationPlugin, time::TimeControlPlugin,
    },
    state::SimulationLifecyclePlugin,
};
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

    // add shared simulation plugins
    app.add_plugins((
        SimulationLifecyclePlugin,
        SimulationAssetPlugin,
        BlockPlugin,
        BiomePlugin,
        ChunkLoadingPlugin,
        TerrainGenerationPlugin,
        TimeControlPlugin,
    ));

    info!("Server started successfully!");
    app.run();
}

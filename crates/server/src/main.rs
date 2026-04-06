use bevy::app::{App, ScheduleRunnerPlugin};
use bevy::prelude::*;
use shared::ecs_core::{LoadingTracker, load_config};
use shared::simulation::app_lifecycle::AppLifecyclePlugin;
use shared::simulation::asset_management::AssetManagementPlugin;
use shared::simulation::biome::BiomePlugin;
use shared::simulation::block::BlockPlugin;
use shared::simulation::chunk::ChunkLoadingPlugin;
use shared::simulation::terrain::TerrainGenerationPlugin;
use shared::simulation::time::TimeControlPlugin;
use std::time::Duration;

fn main() {
    // setup headless bevy app
    let mut app = App::new();
    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        ))),
    );

    // load config & loading trackers into main world
    app.insert_resource(load_config());
    app.insert_resource(LoadingTracker::default());

    // add shared simulation plugins
    app.add_plugins((
        AppLifecyclePlugin,
        AssetManagementPlugin,
        BlockPlugin,
        BiomePlugin,
        ChunkLoadingPlugin,
        TerrainGenerationPlugin,
        TimeControlPlugin,
    ));

    info!("Server started successfully!");
    app.run();
}

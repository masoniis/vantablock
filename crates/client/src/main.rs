use bevy::log::LogPlugin;
use bevy::{app::App, prelude::*, window::WindowResolution};
use client::prelude::*;
use client::render::VantablockRenderPlugin;
use client::simulation::SimulationPlugin;
use shared::ecs_core::LoadingTracker;
use utils::PersistentPaths;

#[instrument(skip_all, fields(name = "main"))]
fn main() {
    attach_logger();

    // setup default bevy app
    let mut app = App::new();

    // Resolve platform paths and initialize application paths
    let persistent_paths = PersistentPaths::resolve();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Vantablock".to_string(),
                    resolution: WindowResolution::new(1280, 720),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: persistent_paths.assets_dir.to_string_lossy().to_string(),
                ..default()
            })
            .disable::<LogPlugin>(),
    );

    // load config & loading trackers into main world
    app.insert_resource(ClientSettings::load_or_create(&persistent_paths));
    app.insert_resource(persistent_paths);
    app.insert_resource(LoadingTracker::default());

    // initialize simulation and renderer
    app.add_plugins(SimulationPlugin);
    app.add_plugins(VantablockRenderPlugin);

    // run the app...
    app.run();
    info!("App exited safely!");
}

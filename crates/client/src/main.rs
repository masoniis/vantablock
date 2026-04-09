#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    app::{App, FixedUpdate, PostUpdate},
    log::LogPlugin,
    prelude::{
        AssetPlugin, ClearColor, Color, DefaultPlugins, IntoScheduleConfigs, PluginGroup, Window,
        WindowPlugin, default, info,
    },
    window::WindowResolution,
};
use client::prelude::*;
use shared::{
    load::LoadingTracker,
    simulation::scheduling::{FixedUpdateSet, RenderPrepSet},
};
use utils::PersistentPaths;

#[instrument(skip_all, fields(name = "main"))]
fn main() {
    attach_logger();

    // setup default bevy app
    let mut app = App::new();

    // resolve platform paths
    let persistent_paths = PersistentPaths::resolve();

    // config of default bevy plugins
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("Vantablock v{}", env!("CARGO_PKG_VERSION")),
                    resolution: WindowResolution::new(1280, 720),
                    visible: false,
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

    // red clear color to prevent white screen flash
    app.insert_resource(ClearColor(Color::linear_rgb(1.0, 0.0, 0.0)));

    // load config & loading trackers into main world
    app.insert_resource(ClientSettings::load_or_create(&persistent_paths));
    app.insert_resource(persistent_paths);
    app.insert_resource(LoadingTracker::default());

    // configure schedule sets
    app.configure_sets(
        FixedUpdate,
        (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).chain(),
    );

    app.configure_sets(PostUpdate, RenderPrepSet);

    // initialize simulation and renderer
    app.add_plugins((shared::SharedPlugins, client::ClientPlugins));

    app.run();
    info!("App exited safely!");
}

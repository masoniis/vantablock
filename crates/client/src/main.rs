#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};

use bevy::{
    app::{App, PostUpdate},
    log::LogPlugin,
    prelude::{default, info, AssetPlugin, DefaultPlugins, PluginGroup, Window, WindowPlugin},
    window::WindowResolution,
};
use client::{lifecycle::scheduling::RenderPrepSet, prelude::*};
use utils::PersistentPaths;

#[instrument(skip_all, fields(name = "main"))]
fn main() {
    attach_logger();

    // setup default bevy app
    let mut app = App::new();

    // resolve platform paths
    let persistent_paths = PersistentPaths::resolve();

    // config of default bevy plugins
    app.add_plugins((
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
        #[cfg(feature = "dev")]
        FpsOverlayPlugin {
            config: FpsOverlayConfig { ..default() },
        },
    ));

    // load config & loading trackers into main world
    app.insert_resource(ClientSettings::load_or_create(&persistent_paths));
    app.insert_resource(persistent_paths);

    app.configure_sets(PostUpdate, RenderPrepSet);

    // initialize simulation and renderer
    app.add_plugins(client::ClientPlugins);

    app.run();
    info!("App exited safely!");
}

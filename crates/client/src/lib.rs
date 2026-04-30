//! # The Vantablock Client Library

pub mod input;
pub mod lifecycle;
pub mod network;
pub mod player;
pub mod prelude;
pub mod render;
pub mod settings;
pub mod showcase;
pub mod ui;

pub use prelude::*;

// INFO: -----------------------------
//         client plugin group
// -----------------------------------

#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::{
    MinimalPlugins,
    asset::AssetApp,
    image::Image,
    input::InputPlugin,
    log::LogPlugin,
    prelude::{AssetPlugin, DefaultPlugins, Window, WindowPlugin, default},
    state::app::StatesPlugin,
    window::WindowResolution,
};

/// A plugin group containing client-side core plugins.
///
/// Notably, this excludes high level bevy plugins meaning
/// that no windows will be spawned, etc. Good for testing.
pub struct CoreClientLogicPlugins;

impl PluginGroup for CoreClientLogicPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // internal crate plugins
            .add(settings::ClientSettingsPlugin)
            .add(input::ClientInputPlugin)
            .add_group(lifecycle::LifecyclePlugins)
            .add(network::ClientNetworkPlugin)
            .add(player::PlayerPlugin)
            .add(showcase::ShowcasePlugin)
    }
}

/// The complete plugin group for the playable game client.
pub struct DefaultClientPlugins;

impl PluginGroup for DefaultClientPlugins {
    fn build(self) -> PluginGroupBuilder {
        let persistent_paths = utils::PersistentPaths::resolve_client();
        let asset_path = persistent_paths.assets_dir.to_string_lossy().to_string();

        #[allow(unused_mut)]
        let mut builder = PluginGroupBuilder::start::<Self>()
            // bevy default setup
            .add_group(
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
                        file_path: asset_path,
                        ..default()
                    })
                    .disable::<LogPlugin>(),
            )
            // external crate plugins
            .add_group(shared::SharedPlugins)
            // rendering plugins, these depend on bevy render app existing
            // so they can't be a part of the core plugin set
            .add(render::VantablockRenderPlugin)
            .add(ui::VantablockUiPlugin)
            // core client
            .add_group(CoreClientLogicPlugins);

        #[cfg(feature = "dev")]
        {
            builder = builder.add(FpsOverlayPlugin {
                config: FpsOverlayConfig { ..default() },
            });
        }

        builder
    }
}

/// The lightweight plugin group for headless integration testing.
pub struct HeadlessClientPlugins;

impl PluginGroup for HeadlessClientPlugins {
    fn build(self) -> PluginGroupBuilder {
        let persistent_paths = utils::PersistentPaths::resolve_client();
        let asset_path = persistent_paths.assets_dir.to_string_lossy().to_string();

        PluginGroupBuilder::start::<Self>()
            // bevy minimal setup
            .add_group(MinimalPlugins)
            // extra bevy plugins not included in minimal
            .add(AssetPlugin {
                file_path: asset_path,
                ..default()
            })
            .add(StatesPlugin)
            .add(InputPlugin)
            // ensure basic assets are registered
            .add(AssetRegistrationPlugin)
            // external crate plugins
            .add_group(shared::SharedPlugins)
            // core client
            .add_group(CoreClientLogicPlugins)
    }
}

struct AssetRegistrationPlugin;

impl Plugin for AssetRegistrationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Image>();
    }
}

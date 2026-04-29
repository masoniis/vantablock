//! # The Vantablock Server Library

pub mod lifecycle;
pub mod network;
pub mod prelude;
pub mod world;

pub use prelude::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{
    app::ScheduleRunnerPlugin, asset::AssetPlugin, prelude::default, state::app::StatesPlugin,
    MinimalPlugins,
};
use shared::SharedPlugins;
use std::time::Duration;

/// Server-side simulation and orchestration plugins.
pub struct ServerCoreLogicPlugins;

impl PluginGroup for ServerCoreLogicPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(lifecycle::ServerLifecyclePlugins)
            .add(network::ServerNetworkPlugin)
            .add(world::ServerWorldPlugin)
    }
}

/// A plugin group containing server related plugins that
/// are intended for a fully-fledged dedicated server.
pub struct DefaultServerPlugins;

impl PluginGroup for DefaultServerPlugins {
    fn build(self) -> PluginGroupBuilder {
        let persistent_paths = utils::PersistentPaths::resolve_client();
        let asset_path = persistent_paths.assets_dir.to_string_lossy().to_string();

        PluginGroupBuilder::start::<Self>()
            // bevy minimal setup
            .add_group(
                MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                    1.0 / 60.0,
                ))),
            )
            .add(StatesPlugin)
            .add(AssetPlugin {
                file_path: asset_path,
                ..default()
            })
            // shared plugin setup
            .add_group(SharedPlugins)
            // server core
            .add_group(ServerCoreLogicPlugins)
    }
}

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

use bevy::app::PluginGroupBuilder;
use bevy::prelude::PluginGroup;
use shared::SharedPlugins;

/// A group containing all client-side plugins.
pub struct ClientPlugins;

impl PluginGroup for ClientPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(lifecycle::LifecyclePlugins)
            .add(network::ClientNetworkPlugin)
            .add(input::InputModulePlugin)
            .add(player::PlayerPlugin)
            .add(showcase::ShowcasePlugin)
            .add(ui::VantablockUiPlugin)
            .add(render::VantablockRenderPlugin)
            // NOTE: shared plugins must come after client network plugin
            // since the protocol must be added after the lightyear `ClientPlugins`
            // https://docs.rs/lightyear/0.26.4/lightyear/prelude/client/struct.ClientPlugins.html
            .add_group(SharedPlugins)
    }
}

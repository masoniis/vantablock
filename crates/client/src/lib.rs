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

/// A group containing all client-side plugins.
pub struct ClientPlugins;

impl PluginGroup for ClientPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(input::InputModulePlugin)
            .add_group(lifecycle::LifecyclePlugins)
            .add(network::ClientNetworkPlugin)
            .add(player::PlayerPlugin)
            .add(render::VantablockRenderPlugin)
            .add(showcase::ShowcasePlugin)
            .add(ui::VantablockUiPlugin)
            // NOTE: shared plugins must come after client network plugin
            // since the protocol must be added after the lightyear `ClientPlugins`
            // https://docs.rs/lightyear/0.26.4/lightyear/prelude/client/struct.ClientPlugins.html
            .add_group(shared::SharedPlugins)
    }
}

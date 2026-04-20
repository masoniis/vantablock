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

/// A plugin group containing every default client-side plugin.
pub struct ClientPlugins;

impl PluginGroup for ClientPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // external crate plugins
            .add_group(shared::SharedPlugins)
            // internal crate plugins
            .add(input::InputModulePlugin)
            .add_group(lifecycle::LifecyclePlugins)
            .add(network::ClientNetworkPlugin)
            .add(player::PlayerPlugin)
            .add(render::VantablockRenderPlugin)
            .add(showcase::ShowcasePlugin)
            .add(ui::VantablockUiPlugin)
    }
}

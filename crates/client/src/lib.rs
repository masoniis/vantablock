pub mod input;
pub mod load;
pub mod player;
pub mod prelude;
pub mod render;
pub mod settings;
pub mod showcase;
pub mod state;
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
            .add(state::ClientLifecyclePlugin)
            .add(load::ClientLoadPlugin)
            .add(input::InputModulePlugin)
            .add(player::PlayerPlugin)
            .add(showcase::ShowcasePlugin)
            .add(ui::VantablockUiPlugin)
            .add(render::VantablockRenderPlugin)
    }
}

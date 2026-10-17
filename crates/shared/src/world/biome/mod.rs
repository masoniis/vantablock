pub mod biome_definition;
pub mod biome_registry;

pub use biome_definition::BiomeDefinition;
pub use biome_registry::{BiomeId, BiomeRegistryResource};

// INFO: ----------------------
//         biome plugin
// ----------------------------

use crate::{
    lifecycle::load::{AppStartupLoadingPhase, LoadBiomes, LoadBlocks, LoadingAppExt},
    world::biome::biome_registry::load_biome_registry,
};
use bevy::app::{App, Plugin};

pub struct BiomePlugin;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut App) {
        app.configure_loading_phase::<AppStartupLoadingPhase>()
            .add_node(LoadBiomes, load_biome_registry)
            .add_dependency(LoadBiomes, LoadBlocks);
    }
}

pub mod biome_definition;
pub mod biome_registry;

pub use biome_definition::BiomeDefinition;
pub use biome_registry::{BiomeId, BiomeRegistryResource};

// INFO: ----------------------
//         Biome plugin
// ----------------------------

use bevy::app::{App, Plugin};

pub struct BiomePlugin;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BiomeRegistryResource>();
    }
}

pub mod biome_definition;
pub mod biome_registry;

pub use biome_definition::BiomeDefinition;
pub use biome_registry::{BiomeId, BiomeRegistryResource};

// INFO: ----------------------
//         Biome plugin
// ----------------------------

use crate::world::biome::biome_registry::initialize_biome_registry_system;
use bevy::app::{App, Plugin, Startup};

pub struct BiomePlugin;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BiomeRegistryResource>();

        app.add_systems(Startup, initialize_biome_registry_system);
    }
}

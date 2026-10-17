pub mod components;
pub mod generators;
pub mod public;
pub mod systems;

pub use components::*;
pub use generators::*;
pub use public::*;

// INFO: ----------------------------
//         terrain gen plugin
// ----------------------------------

use bevy::app::{App, Plugin};
pub use systems::{TerrainGeneratorLibrary, cycle_active_generator};

pub struct TerrainGenerationPlugin;

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClimateNoiseGenerator::new(0)) // hardcode seed 0 for presentation reproducibility
            .insert_resource(ActiveClimateGenerator::default())
            .insert_resource(ActiveBiomeGenerator::default())
            .insert_resource(ActiveTerrainGenerator::default())
            .insert_resource(ActiveTerrainPainter::default())
            .init_resource::<TerrainGeneratorLibrary>();
    }
}

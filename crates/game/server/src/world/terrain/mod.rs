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

use bevy::{
    app::{App, Plugin},
    log::info,
};
use common::world::terrain::WorldGenSettings;
pub use systems::TerrainGeneratorLibrary;

/// Generates terrain as the app's `WorldGenSettings` say. Insert those before adding this
/// plugin (a singleplayer world does), or the defaults are used.
pub struct TerrainGenerationPlugin;

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, app: &mut App) {
        let settings = app
            .world()
            .get_resource::<WorldGenSettings>()
            .copied()
            .unwrap_or_default();
        info!("Generating terrain with {:?}", settings);

        let library = TerrainGeneratorLibrary::default();
        app.insert_resource(ClimateNoiseGenerator::new(0)) // hardcode seed 0 for presentation reproducibility
            .insert_resource(ActiveClimateGenerator::default())
            .insert_resource(ActiveBiomeGenerator::default())
            .insert_resource(ActiveTerrainGenerator(library.get(settings.shaper).clone()))
            .insert_resource(ActiveTerrainPainter::default())
            .insert_resource(library)
            .insert_resource(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::world::terrain::TerrainShaperKind;
    use std::sync::Arc;

    #[test]
    fn generates_with_the_inserted_world_settings() {
        let mut app = App::new();
        app.insert_resource(WorldGenSettings {
            shaper: TerrainShaperKind::Superflat,
        });
        app.add_plugins(TerrainGenerationPlugin);

        let world = app.world();
        let active = &world.resource::<ActiveTerrainGenerator>().0;
        let library = world.resource::<TerrainGeneratorLibrary>();
        assert!(Arc::ptr_eq(
            active,
            library.get(TerrainShaperKind::Superflat)
        ));
    }
}

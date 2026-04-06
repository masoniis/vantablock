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

use crate::prelude::*;
use crate::simulation::input::resources::ActionStateResource;
use crate::simulation::input::types::SimulationAction;
use bevy::app::{App, Plugin, Update};
use bevy::ecs::prelude::{IntoScheduleConfigs, Res};
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

        // INFO: -------------------------------
        //         keybind-based actions
        // -------------------------------------

        app.add_systems(
            Update,
            cycle_active_generator.run_if(|action_state: Res<ActionStateResource>| {
                action_state.just_happened(SimulationAction::CycleActiveTerrainGenerator)
            }),
        );
    }
}

pub mod systems;

use crate::input::resources::ActionStateResource;
use bevy::app::{App, Plugin, Startup, Update};
use bevy::ecs::prelude::{IntoScheduleConfigs, Res};
use shared::simulation::input::types::SimulationAction;
use systems::{apply_default_showcase_system, apply_showcase_system};

pub struct ShowcasePlugin;

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, apply_default_showcase_system);

        app.add_systems(
            Update,
            apply_showcase_system.run_if(|action_state: Res<ActionStateResource>| {
                action_state.just_happened(SimulationAction::Showcase0)
                    || action_state.just_happened(SimulationAction::Showcase1)
                    || action_state.just_happened(SimulationAction::Showcase2)
                    || action_state.just_happened(SimulationAction::Showcase3)
                    || action_state.just_happened(SimulationAction::Showcase4)
                    || action_state.just_happened(SimulationAction::Showcase5)
                    || action_state.just_happened(SimulationAction::Showcase6)
                    || action_state.just_happened(SimulationAction::Showcase7)
                    || action_state.just_happened(SimulationAction::Showcase8)
                    || action_state.just_happened(SimulationAction::Showcase9)
            }),
        );
    }
}

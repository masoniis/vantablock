pub mod camera;
pub mod systems;
pub mod targeted_block;

pub use camera::*;
pub use systems::*;
pub use targeted_block::TargetedBlock;

// INFO: -----------------------
//         player plugin
// -----------------------------

use crate::input::resources::ActionStateResource;
use bevy::app::{App, FixedUpdate, Plugin, Update};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::Res;
use shared::simulation::input::types::simulation_action::SimulationAction;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>();

        app.add_plugins((
            CameraPlugin,
            shared::simulation::player::actions::ActionPlugin,
        ));

        app.add_systems(
            FixedUpdate,
            update_target_voxel::update_targeted_block_system,
        );

        app.add_systems(
            Update,
            (
                voxel_actions::break_targeted_voxel_system.run_if(
                    |action_state: Res<ActionStateResource>| {
                        action_state.just_happened(SimulationAction::BreakVoxel)
                    },
                ),
                voxel_actions::place_targeted_voxel_system.run_if(
                    |action_state: Res<ActionStateResource>| {
                        action_state.just_happened(SimulationAction::PlaceVoxel)
                    },
                ),
            ),
        );
    }
}

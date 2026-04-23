pub mod resources;
pub mod systems;

// INFO: -----------------------------
//         input module plugin
// -----------------------------------

use crate::input::resources::{ActionStateResource, CursorMovement, InputActionMapResource};
use crate::input::systems::toggle_opaque_wireframe::OpaqueRenderMode;
use crate::input::systems::{
    toggle_chunk_borders::ChunkBoundsToggle, toggle_chunk_borders_system, toggle_cursor_system,
    toggle_opaque_wireframe_mode_system,
};
use bevy::app::{App, Plugin, PreUpdate, Update};
use bevy::ecs::{schedule::IntoScheduleConfigs, system::Res};
use bevy::render::extract_resource::ExtractResourcePlugin;
use shared::simulation::input::types::SimulationAction;
use systems::processing;

pub struct InputModulePlugin;

impl Plugin for InputModulePlugin {
    fn build(&self, app: &mut App) {
        // resources
        app.insert_resource(InputActionMapResource::default())
            .insert_resource(ActionStateResource::default());

        app.insert_resource(CursorMovement::default());

        // schedules
        app.add_systems(
            PreUpdate,
            (
                processing::device_events_system,
                processing::update_action_state_system.after(processing::device_events_system),
            ),
        );

        // INFO: -------------------------------------
        //         keybind-based actions below
        // -------------------------------------------

        // set desired cursor state on pause action
        app.add_systems(
            Update,
            toggle_cursor_system.run_if(|action_state: Res<ActionStateResource>| {
                action_state.just_happened(SimulationAction::TogglePause)
            }),
        );

        // toggle opaque wireframe mode
        app.insert_resource(OpaqueRenderMode::default())
            .add_systems(
                Update,
                toggle_opaque_wireframe_mode_system.run_if(
                    |action_state: Res<ActionStateResource>| {
                        action_state.just_happened(SimulationAction::ToggleOpaqueWireframeMode)
                    },
                ),
            );

        // toggle chunk borders
        app.insert_resource(ChunkBoundsToggle::default())
            .add_plugins(ExtractResourcePlugin::<ChunkBoundsToggle>::default())
            .add_systems(
                Update,
                toggle_chunk_borders_system.run_if(|action_state: Res<ActionStateResource>| {
                    action_state.just_happened(SimulationAction::ToggleChunkBorders)
                }),
            );

        // terrain generation cycling
        app.add_systems(
            Update,
            shared::simulation::terrain::cycle_active_generator.run_if(
                |action_state: Res<ActionStateResource>| {
                    action_state.just_happened(SimulationAction::CycleActiveTerrainGenerator)
                },
            ),
        );

        // world clock controls
        app.add_systems(
            Update,
            (
                shared::simulation::time::world_clock::jump_world_clock_backwards_system.run_if(
                    |action_state: Res<ActionStateResource>| {
                        action_state.just_happened(SimulationAction::JumpGameTimeBackward)
                    },
                ),
                shared::simulation::time::world_clock::jump_world_clock_forward_system.run_if(
                    |action_state: Res<ActionStateResource>| {
                        action_state.just_happened(SimulationAction::JumpGameTimeForward)
                    },
                ),
            ),
        );
    }
}

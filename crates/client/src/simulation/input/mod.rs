pub mod systems;

// INFO: -----------------------------
//         input module plugin
// -----------------------------------

use crate::simulation::input::systems::{
    toggle_chunk_borders::ChunkBoundsToggle, toggle_chunk_borders_system, toggle_cursor_system,
    toggle_opaque_wireframe::OpaqueWireframeMode, toggle_opaque_wireframe_mode_system,
};
use bevy::app::{App, Plugin, PreUpdate, Update};
use bevy::ecs::{schedule::IntoScheduleConfigs, system::Res};
use shared::simulation::input::resources::{
    ActionStateResource, CursorMovement, InputActionMapResource,
};
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
                action_state
                    .just_happened(shared::simulation::input::types::SimulationAction::TogglePause)
            }),
        );

        // toggle opaque wireframe mode
        app.insert_resource(OpaqueWireframeMode::default())
            .add_systems(
                Update,
                toggle_opaque_wireframe_mode_system.run_if(
                    |action_state: Res<ActionStateResource>| {
                        action_state.just_happened(
                    shared::simulation::input::types::SimulationAction::ToggleOpaqueWireframeMode,
                )
                    },
                ),
            );

        // toggle chunk borders
        app.insert_resource(ChunkBoundsToggle::default())
            .add_systems(
                Update,
                toggle_chunk_borders_system.run_if(|action_state: Res<ActionStateResource>| {
                    action_state.just_happened(
                        shared::simulation::input::types::SimulationAction::ToggleChunkBorders,
                    )
                }),
            );
    }
}

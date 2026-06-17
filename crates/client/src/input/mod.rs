pub mod input_maps;
pub mod local_actions;
pub mod resources;
pub mod systems;

// INFO: -----------------------------
//         input module plugin
// -----------------------------------

use crate::{
    input::{
        input_maps::get_default_client_action_input_map,
        local_actions::ClientAction,
        resources::CursorMovement,
        systems::{
            lock_cursor_system, toggle_chunk_borders::ChunkBoundsToggle,
            toggle_chunk_borders_system, toggle_opaque_wireframe::OpaqueRenderMode,
            toggle_opaque_wireframe_mode_system, toggle_pause_system, unlock_cursor_system,
        },
    },
    lifecycle::state::{ClientLifecycleState, InGameState},
};
use bevy::{
    app::{App, Plugin, PreUpdate, Update},
    prelude::{IntoScheduleConfigs, OnEnter, OnExit, SystemCondition, in_state},
    render::extract_resource::ExtractResourcePlugin,
};
use leafwing_input_manager::{
    common_conditions::action_just_pressed, plugin::InputManagerPlugin, prelude::ActionState,
};

pub struct ClientInputPlugin;

impl Plugin for ClientInputPlugin {
    fn build(&self, app: &mut App) {
        // leafwing input manager for local actions
        app.add_plugins(InputManagerPlugin::<ClientAction>::default());
        app.init_resource::<ActionState<ClientAction>>()
            .insert_resource(get_default_client_action_input_map());

        // INFO: ------------------------------------
        //         resources & render plugins
        // ------------------------------------------
        app.insert_resource(CursorMovement::default());
        app.insert_resource(OpaqueRenderMode::default());
        app.insert_resource(ChunkBoundsToggle::default());
        app.add_plugins(ExtractResourcePlugin::<ChunkBoundsToggle>::default());

        // INFO: --------------------------------------------
        //         state-transition cursor management
        // --------------------------------------------------

        // cursor management based on game state
        app.add_systems(OnEnter(InGameState::Playing), lock_cursor_system)
            .add_systems(OnExit(InGameState::Playing), unlock_cursor_system)
            .add_systems(
                OnEnter(ClientLifecycleState::MainMenu),
                unlock_cursor_system,
            );

        // INFO: ---------------------------------------
        //         general keybind-based actions
        // ---------------------------------------------

        // ensure device events (mouse movement) are only processed in-game
        app.add_systems(
            PreUpdate,
            systems::processing::device_events_system
                .run_if(in_state(ClientLifecycleState::InGame)),
        );

        // inputs that only run if in game state
        app.add_systems(
            Update,
            (
                // set desired cursor state on pause action
                toggle_pause_system.run_if(
                    action_just_pressed(ClientAction::TogglePause)
                        .and(in_state(InGameState::Playing).or(in_state(InGameState::Paused))),
                ),
                // toggle opaque wireframe mode
                toggle_opaque_wireframe_mode_system
                    .run_if(action_just_pressed(ClientAction::ToggleOpaqueWireframeMode)),
                // toggle chunk borders
                toggle_chunk_borders_system
                    .run_if(action_just_pressed(ClientAction::ToggleChunkBorders)),
            )
                .run_if(in_state(ClientLifecycleState::InGame)),
        );
    }
}

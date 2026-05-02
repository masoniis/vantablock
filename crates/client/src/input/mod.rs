pub mod local_actions;
pub mod resources;
pub mod systems;

// INFO: -----------------------------
//         input module plugin
// -----------------------------------

use crate::{
    input::{
        local_actions::LocalClientAction,
        resources::CursorMovement,
        systems::toggle_opaque_wireframe::OpaqueRenderMode,
        systems::{
            lock_cursor_system, toggle_chunk_borders::ChunkBoundsToggle,
            toggle_chunk_borders_system, toggle_opaque_wireframe_mode_system, toggle_pause_system,
            unlock_cursor_system,
        },
    },
    lifecycle::state::enums::{ClientState, InGameState},
};
use bevy::{
    app::{App, Plugin, PreUpdate, Update},
    prelude::{
        IntoScheduleConfigs, KeyCode, MouseButton, OnEnter, OnExit, SystemCondition, in_state,
    },
    render::extract_resource::ExtractResourcePlugin,
};
use leafwing_input_manager::{
    common_conditions::action_just_pressed, plugin::InputManagerPlugin, prelude::InputMap,
};
use shared::player::PlayerAction;

/// Provides the default input mapping for the game.
pub fn get_default_input_map() -> InputMap<PlayerAction> {
    let mut input_map = InputMap::default();

    // Movement
    input_map.insert(PlayerAction::MoveForward, KeyCode::KeyW);
    input_map.insert(PlayerAction::MoveBackward, KeyCode::KeyS);
    input_map.insert(PlayerAction::MoveLeft, KeyCode::KeyA);
    input_map.insert(PlayerAction::MoveRight, KeyCode::KeyD);
    input_map.insert(PlayerAction::MoveFaster, KeyCode::ShiftLeft);

    // Core player actions
    input_map.insert(PlayerAction::BreakBlock, MouseButton::Left);
    input_map.insert(PlayerAction::PlaceBlock, MouseButton::Right);

    // Terrain gen
    input_map.insert(PlayerAction::CycleActiveTerrainGenerator, KeyCode::KeyT);

    // Game time control
    input_map.insert(PlayerAction::JumpGameTimeForward, KeyCode::ArrowRight);
    input_map.insert(PlayerAction::JumpGameTimeBackward, KeyCode::ArrowLeft);
    input_map.insert(PlayerAction::PauseGameTime, KeyCode::Space);

    input_map
}

/// Provides the default local input mapping for the game.
pub fn get_default_local_input_map() -> InputMap<LocalClientAction> {
    let mut input_map = InputMap::default();

    // Misc
    input_map.insert(LocalClientAction::TogglePause, KeyCode::Escape);

    // Debug/analysis tools
    input_map.insert(LocalClientAction::ToggleDiagnostics, KeyCode::F1);
    input_map.insert(LocalClientAction::ToggleDiagnostics, KeyCode::KeyU);
    input_map.insert(LocalClientAction::ToggleDebugMenu, KeyCode::F3);
    input_map.insert(LocalClientAction::ToggleOpaqueWireframeMode, KeyCode::F2);
    input_map.insert(LocalClientAction::ToggleOpaqueWireframeMode, KeyCode::KeyO);
    input_map.insert(LocalClientAction::ToggleChunkBorders, KeyCode::KeyB);

    // Showcase actions
    input_map.insert(LocalClientAction::Showcase0, KeyCode::Digit0);
    input_map.insert(LocalClientAction::Showcase1, KeyCode::Digit1);
    input_map.insert(LocalClientAction::Showcase2, KeyCode::Digit2);
    input_map.insert(LocalClientAction::Showcase3, KeyCode::Digit3);
    input_map.insert(LocalClientAction::Showcase4, KeyCode::Digit4);
    input_map.insert(LocalClientAction::Showcase5, KeyCode::Digit5);
    input_map.insert(LocalClientAction::Showcase6, KeyCode::Digit6);
    input_map.insert(LocalClientAction::Showcase7, KeyCode::Digit7);
    input_map.insert(LocalClientAction::Showcase8, KeyCode::Digit8);
    input_map.insert(LocalClientAction::Showcase9, KeyCode::Digit9);

    input_map
}

pub struct ClientInputPlugin;

impl Plugin for ClientInputPlugin {
    fn build(&self, app: &mut App) {
        // leafwing input manager for local actions
        app.add_plugins(InputManagerPlugin::<LocalClientAction>::default());

        // Note: InputManagerPlugin for PlayerAction is now handled by SharedPlayerPlugin
        // via lightyear's LeafwingInputPlugin.

        // resources
        app.insert_resource(CursorMovement::default());

        // schedules
        app.add_systems(PreUpdate, (systems::processing::device_events_system,));

        // INFO: -------------------------------------
        //         keybind-based actions below
        // -------------------------------------------

        // set desired cursor state on pause action
        app.add_systems(
            Update,
            toggle_pause_system.run_if(
                action_just_pressed(LocalClientAction::TogglePause)
                    .and(in_state(InGameState::Playing).or(in_state(InGameState::Paused))),
            ),
        );

        // cursor management based on game state
        app.add_systems(OnEnter(InGameState::Playing), lock_cursor_system)
            .add_systems(OnExit(InGameState::Playing), unlock_cursor_system)
            .add_systems(OnEnter(ClientState::MainMenu), unlock_cursor_system);

        // toggle opaque wireframe mode
        app.insert_resource(OpaqueRenderMode::default())
            .add_systems(
                Update,
                toggle_opaque_wireframe_mode_system.run_if(action_just_pressed(
                    LocalClientAction::ToggleOpaqueWireframeMode,
                )),
            );

        // toggle chunk borders
        app.insert_resource(ChunkBoundsToggle::default())
            .add_plugins(ExtractResourcePlugin::<ChunkBoundsToggle>::default())
            .add_systems(
                Update,
                toggle_chunk_borders_system
                    .run_if(action_just_pressed(LocalClientAction::ToggleChunkBorders)),
            );
    }
}

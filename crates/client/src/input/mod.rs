pub mod resources;
pub mod systems;

// INFO: -----------------------------
//         input module plugin
// -----------------------------------

use crate::input::resources::CursorMovement;
use crate::input::systems::toggle_opaque_wireframe::OpaqueRenderMode;
use crate::input::systems::{
    toggle_chunk_borders::ChunkBoundsToggle, toggle_chunk_borders_system, toggle_cursor_system,
    toggle_opaque_wireframe_mode_system,
};
use bevy::app::{App, Plugin, PreUpdate, Update};
use bevy::render::extract_resource::ExtractResourcePlugin;
use bevy::prelude::{KeyCode, MouseButton, IntoScheduleConfigs};
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::InputMap;
use leafwing_input_manager::common_conditions::action_just_pressed;
use shared::simulation::player::PlayerAction;

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
    input_map.insert(PlayerAction::BreakVoxel, MouseButton::Left);
    input_map.insert(PlayerAction::PlaceVoxel, MouseButton::Right);
    
    // Terrain gen
    input_map.insert(PlayerAction::CycleActiveTerrainGenerator, KeyCode::KeyT);
    
    // Game time control
    input_map.insert(PlayerAction::JumpGameTimeForward, KeyCode::ArrowRight);
    input_map.insert(PlayerAction::JumpGameTimeBackward, KeyCode::ArrowLeft);
    input_map.insert(PlayerAction::PauseGameTime, KeyCode::Space);
    
    // Misc
    input_map.insert(PlayerAction::TogglePause, KeyCode::Escape);
    
    // Debug/analysis tools
    input_map.insert(PlayerAction::ToggleDiagnostics, KeyCode::F1);
    input_map.insert(PlayerAction::ToggleDiagnostics, KeyCode::KeyU);
    input_map.insert(PlayerAction::ToggleOpaqueWireframeMode, KeyCode::F2);
    input_map.insert(PlayerAction::ToggleOpaqueWireframeMode, KeyCode::KeyO);
    input_map.insert(PlayerAction::ToggleChunkBorders, KeyCode::F3);
    input_map.insert(PlayerAction::ToggleChunkBorders, KeyCode::KeyB);
    
    // Showcase actions
    input_map.insert(PlayerAction::Showcase0, KeyCode::Digit0);
    input_map.insert(PlayerAction::Showcase1, KeyCode::Digit1);
    input_map.insert(PlayerAction::Showcase2, KeyCode::Digit2);
    input_map.insert(PlayerAction::Showcase3, KeyCode::Digit3);
    input_map.insert(PlayerAction::Showcase4, KeyCode::Digit4);
    input_map.insert(PlayerAction::Showcase5, KeyCode::Digit5);
    input_map.insert(PlayerAction::Showcase6, KeyCode::Digit6);
    input_map.insert(PlayerAction::Showcase7, KeyCode::Digit7);
    input_map.insert(PlayerAction::Showcase8, KeyCode::Digit8);
    input_map.insert(PlayerAction::Showcase9, KeyCode::Digit9);
    
    input_map
}

pub struct InputModulePlugin;

impl Plugin for InputModulePlugin {
    fn build(&self, app: &mut App) {
        // leafwing input manager
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default());

        // resources
        app.insert_resource(CursorMovement::default());

        // schedules
        app.add_systems(
            PreUpdate,
            (
                systems::processing::device_events_system,
            ),
        );

        // INFO: -------------------------------------
        //         keybind-based actions below
        // -------------------------------------------

        // set desired cursor state on pause action
        app.add_systems(
            Update,
            toggle_cursor_system.run_if(action_just_pressed(PlayerAction::TogglePause)),
        );

        // toggle opaque wireframe mode
        app.insert_resource(OpaqueRenderMode::default())
            .add_systems(
                Update,
                toggle_opaque_wireframe_mode_system
                    .run_if(action_just_pressed(PlayerAction::ToggleOpaqueWireframeMode)),
            );

        // toggle chunk borders
        app.insert_resource(ChunkBoundsToggle::default())
            .add_plugins(ExtractResourcePlugin::<ChunkBoundsToggle>::default())
            .add_systems(
                Update,
                toggle_chunk_borders_system
                    .run_if(action_just_pressed(PlayerAction::ToggleChunkBorders)),
            );
    }
}

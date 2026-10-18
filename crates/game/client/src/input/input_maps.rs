use crate::input::local_actions::ClientAction;
use bevy::prelude::KeyCode;
use common::player::PlayerAction;
use leafwing_input_manager::prelude::InputMap;
use leafwing_input_manager::user_input::mouse::MouseScrollAxis;

/// Provides the default input mapping for the game.
pub fn get_default_player_action_input_map() -> InputMap<PlayerAction> {
    let mut input_map = InputMap::default();

    // movement
    input_map.insert(PlayerAction::MoveForward, KeyCode::KeyW);
    input_map.insert(PlayerAction::MoveBackward, KeyCode::KeyS);
    input_map.insert(PlayerAction::MoveLeft, KeyCode::KeyA);
    input_map.insert(PlayerAction::MoveRight, KeyCode::KeyD);
    input_map.insert(PlayerAction::MoveUp, KeyCode::Space);
    input_map.insert(PlayerAction::MoveDown, KeyCode::ControlLeft);
    input_map.insert(PlayerAction::MoveFaster, KeyCode::ShiftLeft);

    input_map
}

/// Provides the default local input mapping for the game.
pub fn get_default_client_action_input_map() -> InputMap<ClientAction> {
    let mut input_map = InputMap::default();

    // continuous inputs
    input_map.insert_axis(ClientAction::Zoom, MouseScrollAxis::Y);

    // menus
    input_map.insert(ClientAction::TogglePause, KeyCode::Escape);
    input_map.insert(ClientAction::ToggleDebugMenu, KeyCode::F3);

    // renderer inspection
    input_map.insert(ClientAction::CycleDebugView, KeyCode::F2);
    input_map.insert(ClientAction::CycleDebugView, KeyCode::KeyV);
    input_map.insert(ClientAction::ToggleSceneAcceleration, KeyCode::F4);
    input_map.insert(ClientAction::ToggleSceneAcceleration, KeyCode::KeyB);
    input_map.insert(ClientAction::CycleRenderScale, KeyCode::F5);
    input_map.insert(ClientAction::ToggleLocalHierarchy, KeyCode::F6);
    input_map.insert(ClientAction::ToggleLocalHierarchy, KeyCode::KeyH);

    // world clock
    input_map.insert(ClientAction::TimeForward, KeyCode::ArrowRight);
    input_map.insert(ClientAction::TimeBackward, KeyCode::ArrowLeft);
    input_map.insert(ClientAction::ToggleDayCycle, KeyCode::KeyP);

    input_map
}

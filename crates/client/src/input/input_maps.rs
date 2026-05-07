use crate::input::local_actions::ClientAction;
use bevy::prelude::{KeyCode, MouseButton};
use leafwing_input_manager::prelude::{InputMap, MouseMove};
use shared::player::PlayerAction;

/// Provides the default input mapping for the game.
pub fn get_default_player_action_input_map() -> InputMap<PlayerAction> {
    let mut input_map = InputMap::default();

    // movement
    input_map.insert(PlayerAction::MoveForward, KeyCode::KeyW);
    input_map.insert(PlayerAction::MoveBackward, KeyCode::KeyS);
    input_map.insert(PlayerAction::MoveLeft, KeyCode::KeyA);
    input_map.insert(PlayerAction::MoveRight, KeyCode::KeyD);
    input_map.insert(PlayerAction::MoveFaster, KeyCode::ShiftLeft);

    // core player actions
    input_map.insert(PlayerAction::PrimaryInteract, MouseButton::Left);
    input_map.insert(PlayerAction::SecondaryInteract, MouseButton::Right);

    // game time control
    input_map.insert(PlayerAction::JumpGameTimeForward, KeyCode::ArrowRight);
    input_map.insert(PlayerAction::JumpGameTimeBackward, KeyCode::ArrowLeft);
    input_map.insert(PlayerAction::PauseGameTime, KeyCode::Space);

    input_map
}

/// Provides the default local input mapping for the game.
pub fn get_default_client_action_input_map() -> InputMap<ClientAction> {
    let mut input_map = InputMap::default();

    // orientation
    input_map.insert_dual_axis(ClientAction::Look, MouseMove::default());

    // misc
    input_map.insert(ClientAction::TogglePause, KeyCode::Escape);

    // debug/analysis tools
    input_map.insert(ClientAction::ToggleDiagnostics, KeyCode::F1);
    input_map.insert(ClientAction::ToggleDiagnostics, KeyCode::KeyU);
    input_map.insert(ClientAction::ToggleDebugMenu, KeyCode::F3);
    input_map.insert(ClientAction::ToggleOpaqueWireframeMode, KeyCode::F2);
    input_map.insert(ClientAction::ToggleOpaqueWireframeMode, KeyCode::KeyO);
    input_map.insert(ClientAction::ToggleChunkBorders, KeyCode::KeyB);

    // showcase actions
    input_map.insert(ClientAction::Showcase0, KeyCode::Digit0);
    input_map.insert(ClientAction::Showcase1, KeyCode::Digit1);
    input_map.insert(ClientAction::Showcase2, KeyCode::Digit2);
    input_map.insert(ClientAction::Showcase3, KeyCode::Digit3);
    input_map.insert(ClientAction::Showcase4, KeyCode::Digit4);
    input_map.insert(ClientAction::Showcase5, KeyCode::Digit5);
    input_map.insert(ClientAction::Showcase6, KeyCode::Digit6);
    input_map.insert(ClientAction::Showcase7, KeyCode::Digit7);
    input_map.insert(ClientAction::Showcase8, KeyCode::Digit8);
    input_map.insert(ClientAction::Showcase9, KeyCode::Digit9);

    input_map
}

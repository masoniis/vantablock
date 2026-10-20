use crate::input::local_actions::ClientAction;
use bevy::prelude::KeyCode;
use common::player::PlayerAction;
use leafwing_input_manager::{prelude::InputMap, user_input::mouse::MouseScrollAxis};

/// Default keys for the networked player actions. The pause menu's controls list reads these, so
/// it always shows what is actually bound.
pub const PLAYER_KEYS: &[(PlayerAction, &[KeyCode])] = &[
    (PlayerAction::MoveForward, &[KeyCode::KeyW]),
    (PlayerAction::MoveBackward, &[KeyCode::KeyS]),
    (PlayerAction::MoveLeft, &[KeyCode::KeyA]),
    (PlayerAction::MoveRight, &[KeyCode::KeyD]),
    (PlayerAction::MoveUp, &[KeyCode::Space]),
    (PlayerAction::MoveDown, &[KeyCode::ControlLeft]),
    (PlayerAction::MoveFaster, &[KeyCode::ShiftLeft]),
];

/// Default keys for the client-only actions (`Zoom` is the mouse wheel, bound separately).
pub const CLIENT_KEYS: &[(ClientAction, &[KeyCode])] = &[
    // menus
    (ClientAction::TogglePause, &[KeyCode::Escape]),
    (ClientAction::ToggleDebugMenu, &[KeyCode::F3]),
    // renderer inspection
    (ClientAction::CycleDebugView, &[KeyCode::F2, KeyCode::KeyV]),
    (
        ClientAction::ToggleSceneAcceleration,
        &[KeyCode::F4, KeyCode::KeyB],
    ),
    (ClientAction::CycleRenderScale, &[KeyCode::F5]),
    (
        ClientAction::ToggleLocalHierarchy,
        &[KeyCode::F6, KeyCode::KeyH],
    ),
    (
        ClientAction::ToggleHardwareBroadPhase,
        &[KeyCode::F7, KeyCode::KeyG],
    ),
    (
        ClientAction::ToggleFootprintLod,
        &[KeyCode::F8, KeyCode::KeyL],
    ),
    // world clock
    (ClientAction::TimeForward, &[KeyCode::ArrowRight]),
    (ClientAction::TimeBackward, &[KeyCode::ArrowLeft]),
    (ClientAction::ToggleDayCycle, &[KeyCode::KeyP]),
];

/// The default keys bound to a player action.
pub fn player_keys(action: PlayerAction) -> &'static [KeyCode] {
    keys_in(PLAYER_KEYS, action)
}

/// The default keys bound to a client action.
pub fn client_keys(action: ClientAction) -> &'static [KeyCode] {
    keys_in(CLIENT_KEYS, action)
}

fn keys_in<A: PartialEq>(
    table: &'static [(A, &'static [KeyCode])],
    action: A,
) -> &'static [KeyCode] {
    table
        .iter()
        .find(|(a, _)| *a == action)
        .map_or(&[], |(_, keys)| keys)
}

/// Provides the default input mapping for the game.
pub fn get_default_player_action_input_map() -> InputMap<PlayerAction> {
    let mut input_map = InputMap::default();
    for &(action, keys) in PLAYER_KEYS {
        for &key in keys {
            input_map.insert(action, key);
        }
    }
    input_map
}

/// Provides the default local input mapping for the game.
pub fn get_default_client_action_input_map() -> InputMap<ClientAction> {
    let mut input_map = InputMap::default();
    input_map.insert_axis(ClientAction::Zoom, MouseScrollAxis::Y);
    for &(action, keys) in CLIENT_KEYS {
        for &key in keys {
            input_map.insert(action, key);
        }
    }
    input_map
}

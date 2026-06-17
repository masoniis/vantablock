//! # Vantablock UI Library
//!
//! This module is for reusable ui stuff and general screens. Other UI may exist within modules
//! that are more relevant to them (like inventory and crosshair).

mod screens;
pub mod systems;
mod widgets;

// INFO: -------------------
//         ui plugin
// -------------------------

use crate::lifecycle::state::ClientLifecycleState;
use bevy::prelude::*;

pub struct VantablockUiPlugin;

impl Plugin for VantablockUiPlugin {
    fn build(&self, app: &mut App) {
        // screen plugins
        app.add_plugins((
            screens::LaunchingClientScreenPlugin,
            screens::main_menu::MainMenuUiPlugin,
            screens::connecting::ConnectingUiPlugin,
            screens::settings::SettingsUiPlugin,
            screens::disconnected::DisconnectedUiPlugin,
            screens::debug_menu::DebugMenuPlugin,
        ));

        // game ui
        app.add_systems(
            OnEnter(ClientLifecycleState::InGame),
            systems::despawn_menu_camera_system,
        );
    }
}

//! # Vantablock UI Library
//!
//! This module is for reusable ui stuff and general screens. Other UI may exist within modules
//! that are more relevant to them (like inventory and crosshair).
//!
//! Screens are `bsn!` scenes composed from [`theme`] (palette, fonts, text) and [`widgets`]
//! (buttons, text fields, keycaps), with behavior attached through `on(...)` observers.

pub mod screens;
pub mod systems;
pub mod theme;
pub mod widgets;

// INFO: -------------------
//         ui plugin
// -------------------------

use crate::lifecycle::state::InGameState;
use bevy::prelude::*;

pub struct VantablockUiPlugin;

impl Plugin for VantablockUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(widgets::WidgetsPlugin);

        // screen plugins
        app.add_plugins((
            screens::LaunchingClientScreenPlugin,
            screens::main_menu::MainMenuUiPlugin,
            screens::connecting::ConnectingUiPlugin,
            screens::pause_menu::PauseMenuPlugin,
            screens::disconnected::DisconnectedUiPlugin,
            screens::debug_menu::DebugMenuPlugin,
        ));

        // the menu camera renders the connecting screen, then gives way to the player camera
        app.add_systems(
            OnExit(InGameState::Connecting),
            systems::despawn_menu_camera_system,
        );
    }
}

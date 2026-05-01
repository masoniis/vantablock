//! # Vantablock UI Library
//!
//! This module is for reusable ui stuff and general screens. Other UI may exist within modules
//! that are more relevant to them (like inventory and crosshair).

pub mod root;

// INFO: -------------------
//         ui plugin
// -------------------------

mod screens;

use crate::lifecycle::state::enums::InGameState;
use crate::lifecycle::{SimulationState, state::ClientState};
use bevy::prelude::*;
use shared::lifecycle::state::enums::AppState;

pub struct VantablockUiPlugin;

impl Plugin for VantablockUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(screens::debug_menu::DebugMenuPlugin);

        app.add_systems(OnEnter(AppState::Running), root::spawn_ui_root)
            .add_systems(OnExit(AppState::Running), root::despawn_ui_root);

        // starting up ui
        app.add_systems(
            OnEnter(SimulationState::Loading),
            screens::starting_up_ui::spawn_starting_up_ui,
        );

        // main menu ui
        app.add_systems(
            OnEnter(ClientState::MainMenu),
            screens::main_menu::spawn_main_menu,
        )
        .add_systems(
            Update,
            (
                screens::main_menu::main_menu_button_interaction_system,
                screens::main_menu::main_menu_text_input_system,
            )
                .run_if(in_state(ClientState::MainMenu)),
        );

        // connecting ui
        app.add_systems(
            OnEnter(InGameState::Connecting),
            screens::connecting::spawn_connecting_ui,
        );

        // settings ui
        app.add_systems(
            OnEnter(InGameState::Paused),
            screens::settings::spawn_settings_ui,
        )
        .add_systems(
            Update,
            screens::settings::settings_button_interaction_system
                .run_if(in_state(InGameState::Paused)),
        );

        // disconnected ui
        // it is spawned via trigger NetworkErrorEvent
        app.add_observer(screens::disconnected::spawn_disconnected_ui);

        app.add_systems(
            Update,
            screens::disconnected::disconnected_ui_button_interaction_system
                .run_if(in_state(ClientState::Error)),
        );
    }
}

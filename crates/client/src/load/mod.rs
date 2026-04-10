pub mod registries;

use crate::load::registries::start_async_registry_initialization;
use bevy::app::{App, Plugin};
use bevy::prelude::OnEnter;
use shared::lifecycle::state::enums::AppState;

/// Plugin responsible for managing client-side asset and registry loading.
pub struct ClientLoadPlugin;

impl Plugin for ClientLoadPlugin {
    fn build(&self, app: &mut App) {
        // start background registry initialization
        app.add_systems(
            OnEnter(AppState::StartingUp),
            start_async_registry_initialization,
        );
    }
}

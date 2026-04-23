pub mod chunk_loading;
pub mod registries;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, PreUpdate};
use bevy::ecs::query::{Changed, With};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::Query;
use bevy::prelude::{Camera, Camera3d, OnEnter};
use chunk_loading::manage_distance_based_chunk_loading_targets_system;
use registries::start_async_registry_initialization;
use shared::lifecycle::state::enums::AppState;
use shared::simulation::chunk::ChunkCoord;

/// Plugin responsible for managing client-side asset and registry loading.
pub struct ClientLoadPlugin;

impl Plugin for ClientLoadPlugin {
    fn build(&self, app: &mut App) {
        // start background registry initialization
        app.add_systems(
            OnEnter(AppState::StartingUp),
            start_async_registry_initialization,
        );

        app.add_systems(
            PreUpdate,
            (manage_distance_based_chunk_loading_targets_system).run_if(
                |q: Query<(&Camera, &ChunkCoord), (With<Camera3d>, Changed<ChunkCoord>)>| {
                    q.iter().any(|(c, _)| c.is_active)
                },
            ),
        );
    }
}

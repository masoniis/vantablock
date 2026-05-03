pub mod chunk_loading;
pub mod registries;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use chunk_loading::manage_distance_based_chunk_loading_targets_system;
use registries::start_async_registry_initialization;
use shared::{
    SimulationLoadingPhase, cleanup_orphaned_tasks, lifecycle::state::enums::AppState,
    loading_is_complete, poll_tasks, start_fake_work_system, transition_to,
    world::chunk::ChunkCoord,
};

use crate::lifecycle::SimulationState;

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

        // TODO: remove temp fake work system for testing
        app.add_systems(OnEnter(SimulationState::Loading), start_fake_work_system);

        // polling systems and tracking load state for simulation loading
        app.add_systems(
            Update,
            (
                poll_tasks::<SimulationLoadingPhase>,
                transition_to(SimulationState::Running)
                    .run_if(loading_is_complete::<SimulationLoadingPhase>),
            )
                .chain()
                .run_if(in_state(SimulationState::Loading)),
        )
        .add_systems(
            OnExit(SimulationState::Loading),
            cleanup_orphaned_tasks::<SimulationLoadingPhase>,
        );
    }
}

pub mod chunk_loading;
pub mod phases;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::ClientLifecycleState;
use bevy::prelude::*;
use chunk_loading::manage_distance_based_chunk_loading_targets_system;
pub use phases::*;
use shared::{
    lifecycle::{
        load::{kickoff_loading_phase, loading_dag_is_complete, nuke_loading_dag},
        state::transition_to,
    },
    world::chunk::ChunkCoord,
};

/// Plugin responsible for managing client-side asset and registry loading.
pub struct ClientLoadPlugin;

impl Plugin for ClientLoadPlugin {
    fn build(&self, app: &mut App) {
        // kickoff client launch loading phase when launching
        app.add_systems(
            OnEnter(ClientLifecycleState::Launching),
            kickoff_loading_phase::<ClientLaunchLoadingPhase>,
        );

        // transition to main menu when client launch tasks are complete
        app.add_systems(
            Update,
            transition_to(ClientLifecycleState::MainMenu)
                .run_if(in_state(ClientLifecycleState::Launching))
                .run_if(loading_dag_is_complete::<ClientLaunchLoadingPhase>),
        );

        // cleanup
        app.add_systems(
            OnExit(ClientLifecycleState::Launching),
            nuke_loading_dag::<ClientLaunchLoadingPhase>,
        );

        app.add_systems(
            PreUpdate,
            manage_distance_based_chunk_loading_targets_system.run_if(
                |q: Query<(&Camera, &ChunkCoord), (With<Camera3d>, Changed<ChunkCoord>)>| {
                    q.iter().any(|(c, _)| c.is_active)
                },
            ),
        );
    }
}

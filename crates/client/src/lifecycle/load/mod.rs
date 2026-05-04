pub mod chunk_loading;
pub mod loading_phases;
pub mod registries;

pub use loading_phases::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use chunk_loading::manage_distance_based_chunk_loading_targets_system;
use registries::{
    handle_biome_loading, handle_block_loading, handle_render_registry, handle_texture_stitching,
};
use shared::{
    cleanup_orphaned_tasks, kickoff_loading_phase,
    lifecycle::state::{enums::AppState, transition_to},
    loading_dag_is_complete, nuke_loading_dag, poll_tasks, reset_loading_dag_state,
    start_fake_work_system,
    world::chunk::ChunkCoord,
    LoadingAppExt, LoadingTaskComponent, NodeFinished, StartNode,
};

use crate::lifecycle::SimulationState;

/// Plugin responsible for managing client-side asset and registry loading.
pub struct ClientLoadPlugin;

impl Plugin for ClientLoadPlugin {
    fn build(&self, app: &mut App) {
        // configure async loading DAG for app startup
        app.configure_loading_phase::<AppStartupPhase>()
            .add_node(AppStartupPhase::Textures, handle_texture_stitching)
            .add_node(AppStartupPhase::Blocks, handle_block_loading)
            .add_node(AppStartupPhase::Biomes, handle_biome_loading)
            .add_node(AppStartupPhase::RenderRegistry, handle_render_registry)
            .add_edge(AppStartupPhase::Textures, AppStartupPhase::RenderRegistry)
            .add_edge(AppStartupPhase::Blocks, AppStartupPhase::RenderRegistry)
            .add_edge(AppStartupPhase::Biomes, AppStartupPhase::RenderRegistry);

        // kickoff the app startup loading phase when starting up
        app.add_systems(
            OnEnter(AppState::StartingUp),
            kickoff_loading_phase::<AppStartupPhase>,
        )
        .add_systems(
            OnExit(AppState::StartingUp),
            nuke_loading_dag::<AppStartupPhase>,
        );

        // handle transition to running state when app startup is done
        app.add_systems(
            Update,
            (
                poll_tasks::<AppStartupPhase>,
                transition_to(AppState::Running).run_if(loading_dag_is_complete::<AppStartupPhase>),
            )
                .chain()
                .run_if(in_state(AppState::StartingUp)),
        );

        app.add_systems(
            PreUpdate,
            (manage_distance_based_chunk_loading_targets_system).run_if(
                |q: Query<(&Camera, &ChunkCoord), (With<Camera3d>, Changed<ChunkCoord>)>| {
                    q.iter().any(|(c, _)| c.is_active)
                },
            ),
        );

        // configure async loading DAG for simulation loading
        app.configure_loading_phase::<SimulationLoadingPhase>()
            .add_node(
                SimulationLoadingPhase::FakeWork,
                |trigger: On<StartNode<SimulationLoadingPhase>>, mut commands: Commands| {
                    info!("Starting simulation fake work node!");
                    let entity = trigger.event().entity;

                    let task = start_fake_work_system();

                    let wrapped_task = AsyncComputeTaskPool::get().spawn(async move {
                        let mut queue = task.await;

                        queue.push(move |world: &mut World| {
                            world.trigger(NodeFinished {
                                node: SimulationLoadingPhase::FakeWork,
                                entity,
                            });
                        });

                        queue
                    });

                    commands.spawn((
                        LoadingTaskComponent(wrapped_task),
                        SimulationLoadingPhase::FakeWork,
                    ));
                },
            );

        // kickoff simulation loading when entering Loading state
        app.add_systems(
            OnEnter(SimulationState::Loading),
            kickoff_loading_phase::<SimulationLoadingPhase>,
        );

        // polling systems and tracking load state for simulation loading
        app.add_systems(
            Update,
            (
                poll_tasks::<SimulationLoadingPhase>,
                transition_to(SimulationState::Running)
                    .run_if(loading_dag_is_complete::<SimulationLoadingPhase>),
            )
                .chain()
                .run_if(in_state(SimulationState::Loading)),
        )
        .add_systems(
            OnExit(SimulationState::Loading),
            (
                cleanup_orphaned_tasks::<SimulationLoadingPhase>,
                reset_loading_dag_state::<SimulationLoadingPhase>,
            ),
        );
    }
}

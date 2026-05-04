pub mod loading_phases;

use bevy::{ecs::world::CommandQueue, prelude::*, tasks::AsyncComputeTaskPool};
pub use loading_phases::*;
use shared::{
    lifecycle::{
        PersistentPathsResource,
        load::{
            LoadingAppExt, LoadingTaskComponent, NodeFinished, StartNode, cleanup_orphaned_tasks,
            kickoff_loading_phase, loading_dag_is_complete, poll_tasks,
        },
        state::{enums::AppState, transition_to},
    },
    world::{biome::BiomeRegistryResource, block::BlockRegistry},
};

use crate::lifecycle::state::ServerState;

pub struct ServerLoadPlugin;

impl Plugin for ServerLoadPlugin {
    fn build(&self, app: &mut App) {
        // immediately transition from StartingUp to Running on server
        // (server doesn't have AppStartupLoadingPhase rendering tasks)
        app.add_systems(
            Update,
            transition_to(AppState::Running).run_if(in_state(AppState::StartingUp)),
        );

        // configure async loading DAG for server startup
        app.configure_loading_phase::<SimulationLoadingPhase>()
            .add_node(SimulationLoadingPhase::Biomes, handle_biome_loading)
            .add_node(SimulationLoadingPhase::Blocks, handle_block_loading);

        // kickoff loading when entering Initializing state
        app.add_systems(
            OnEnter(ServerState::Initializing),
            kickoff_loading_phase::<SimulationLoadingPhase>,
        );

        // handle transition to running state when server initialization is done
        app.add_systems(
            Update,
            (
                poll_tasks::<SimulationLoadingPhase>,
                transition_to_running.run_if(loading_dag_is_complete::<SimulationLoadingPhase>),
            )
                .chain()
                .run_if(in_state(ServerState::Initializing)),
        )
        .add_systems(
            OnExit(ServerState::Initializing),
            cleanup_orphaned_tasks::<SimulationLoadingPhase>,
        );
    }
}

fn transition_to_running(mut server_state: ResMut<NextState<ServerState>>) {
    server_state.set(ServerState::Running);
}

/// Observer that handles the biome registry loading task.
pub fn handle_biome_loading(
    _trigger: On<StartNode<SimulationLoadingPhase>>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
) {
    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let biome_registry = BiomeRegistryResource::load_from_disk(&paths);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(biome_registry);
            world.trigger(NodeFinished(SimulationLoadingPhase::Biomes));
        });
        queue
    });

    commands.spawn((LoadingTaskComponent(task), SimulationLoadingPhase::Biomes));
}

/// Observer that handles the block registry loading task.
pub fn handle_block_loading(
    _trigger: On<StartNode<SimulationLoadingPhase>>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
) {
    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let block_registry = BlockRegistry::load_from_disk(&paths);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(block_registry);
            world.trigger(NodeFinished(SimulationLoadingPhase::Blocks));
        });
        queue
    });

    commands.spawn((LoadingTaskComponent(task), SimulationLoadingPhase::Blocks));
}

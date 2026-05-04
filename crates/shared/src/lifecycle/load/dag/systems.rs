use crate::lifecycle::load::dag::{
    components::{LoadingDagPhase, LoadingTaskComponent, NodeFinished, PhaseFinished, StartNode},
    resource::LoadingDag,
};
use bevy::prelude::*;
use bevy::tasks::{block_on, poll_once};

/// A generic system that automatically identifies and kicks off all root nodes
/// (those with no dependencies) for a specific loading phase.
pub fn kickoff_loading_phase<N: LoadingDagPhase>(dag: Res<LoadingDag<N>>, mut commands: Commands) {
    let phase_name = std::any::type_name::<N>()
        .split("::")
        .last()
        .unwrap_or("Unknown");

    if dag.dependencies.is_empty() {
        warn!(
            "[{}] Phase is empty! Transitioning immediately.",
            phase_name
        );
        commands.trigger(PhaseFinished::<N>(std::marker::PhantomData));
        return;
    }

    info!("[{}] Kicking off loading phase DAG...", phase_name);

    for (node, deps) in &dag.dependencies {
        // a node is a root if its dependency list is empty
        if deps.is_empty() {
            // trigger the global observer for this specific node
            commands.trigger(StartNode(*node));
        }
    }
}

/// Coordinator observer that evaluates dependencies whenever a node finishes.
///
/// This system listens for `NodeFinished<N>` globally and triggers the next ready nodes.
pub fn evaluate_dag_dependencies<N: LoadingDagPhase>(
    event: On<NodeFinished<N>>,
    mut commands: Commands,
    mut dag: ResMut<LoadingDag<N>>,
) {
    let finished_node = event.event().0;

    if !dag.completed_nodes.contains(&finished_node) {
        dag.completed_nodes.push(finished_node);
    }

    // calculate and log progress
    let completed = dag.completed_nodes.len();
    let total = dag.dependencies.len();
    let phase_name = std::any::type_name::<N>()
        .split("::")
        .last()
        .unwrap_or("UnknownPhase");

    info!("[{}] Progress: {}/{}", phase_name, completed, total);

    // check if the entire phase is complete
    if completed == total && total > 0 {
        info!("[{}] Phase complete!", phase_name);
        commands.trigger(PhaseFinished::<N>(std::marker::PhantomData));
    }

    // find nodes that have all their dependencies met and haven't started yet
    let mut nodes_to_start = Vec::new();

    for (node, deps) in &dag.dependencies {
        if dag.started_nodes.contains(node) {
            continue;
        }

        let all_deps_met = deps.iter().all(|dep| dag.completed_nodes.contains(dep));
        if all_deps_met {
            nodes_to_start.push(*node);
        }
    }

    // trigger StartNode for each ready node
    for node in nodes_to_start {
        dag.started_nodes.push(node);

        // trigger globally
        commands.trigger(StartNode(node));
    }
}

/// A generic polling system that checks if tasks of a specific marker have finished.
///
/// When a task finishes, this system applies the tasks' returned commands and despawns
/// the task entity.
pub fn poll_tasks<N: LoadingDagPhase>(
    mut tasks: Query<(Entity, &mut LoadingTaskComponent), With<N>>,
    mut commands: Commands,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(mut queue) = block_on(poll_once(&mut task.0)) {
            commands.append(&mut queue);
            // use try_despawn to avoid crashing if the entity was already despawned
            // by another system or a previous command in this turn
            commands.entity(entity).try_despawn();
        }
    }
}

/// A generic cleanup system for orphaned tasks.
pub fn cleanup_orphaned_tasks<N: LoadingDagPhase>(
    mut commands: Commands,
    query: Query<Entity, (With<LoadingTaskComponent>, With<N>)>,
) {
    for entity in &query {
        warn!(
            "Cleaning up orphaned task for node type: {}",
            std::any::type_name::<N>()
        );
        commands.entity(entity).try_despawn();
    }
}

/// Returns true if the DAG for the specified node type is complete.
pub fn loading_dag_is_complete<N: LoadingDagPhase>(dag: Option<Res<LoadingDag<N>>>) -> bool {
    dag.is_some_and(|d| d.completed_nodes.len() == d.dependencies.len())
}

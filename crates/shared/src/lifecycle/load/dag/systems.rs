use crate::lifecycle::load::dag::{
    components::{LoadingDagPhase, LoadingTaskComponent, NodeCompleted, PhaseFinished, StartNode},
    resource::LoadingDag,
};
use bevy::{
    prelude::*,
    tasks::{block_on, poll_once},
};

/// A generic system that automatically identifies and kicks off all root nodes
/// (those with no dependencies) for a specific loading phase.
pub fn kickoff_loading_phase<P: LoadingDagPhase>(
    dag: Option<ResMut<LoadingDag<P>>>,
    mut commands: Commands,
) {
    let Some(mut dag) = dag else {
        info!(
            "[{}] Phase is not configured (no tasks registered). Transitioning immediately.",
            P::PHASE_NAME
        );
        commands.trigger(PhaseFinished::<P>(std::marker::PhantomData));
        return;
    };

    if !dag.started_nodes.is_empty() {
        warn!(
            "[{}] Started nodes is not empty but kickoff was called!",
            P::PHASE_NAME
        );
    }

    if !dag.completed_nodes.is_empty() {
        warn!(
            "[{}] Completed nodes is not empty but kickoff was called!",
            P::PHASE_NAME
        );
    }

    if dag.dependencies.is_empty() {
        warn!(
            "[{}] Phase is empty! Transitioning immediately.",
            P::PHASE_NAME
        );
        commands.trigger(PhaseFinished::<P>(std::marker::PhantomData));
        return;
    }

    info!("[{}] Kicking off loading phase DAG...", P::PHASE_NAME);

    let mut nodes_to_start = Vec::new();

    for (node, deps) in &dag.dependencies {
        // a phase node is a root if its dependency list is empty
        if deps.is_empty() {
            nodes_to_start.push(*node);
        }
    }

    for node in nodes_to_start {
        dag.started_nodes.push(node);
        // trigger targeted observer for this specific phase node entity
        if let Some(entity) = dag.node_entities.get(&node) {
            commands.entity(*entity).trigger(StartNode);
        } else {
            warn!(
                "[{}] Attempted to start node '{:?}' without a registered entity",
                P::PHASE_NAME,
                node
            );
        }
    }
}

/// Coordinator observer that evaluates dependencies whenever a phase node finishes.
///
/// This system listens for `NodeCompleted` globally and triggers the next ready phase nodes.
pub fn evaluate_dag_dependencies<P: LoadingDagPhase>(
    event: On<NodeCompleted>,
    mut commands: Commands,
    mut dag: ResMut<LoadingDag<P>>,
) {
    let finished_node = event.event().node_type;

    // we only care about nodes that are part of THIS DAG
    if !dag.dependencies.contains_key(&finished_node) {
        return;
    }

    if !dag.started_nodes.contains(&finished_node) {
        warn!(
            "Dag node completed but hasn't started. This likely means the node was duplicated accross multiple loading DAGs which is not an advised pattern. Ignoring the completion..."
        );
        return;
    }

    if !dag.completed_nodes.contains(&finished_node) {
        dag.completed_nodes.push(finished_node);

        // calculate and log progress
        let completed = dag.completed_nodes.len();
        let total = dag.dependencies.len();

        info!(
            "[{}] Progress: {}/{} (Finished {})",
            P::PHASE_NAME,
            completed,
            total,
            event.event().node_name,
        );

        // check if the entire phase is complete
        if completed == total && total > 0 {
            info!("[{}] Phase complete!", P::PHASE_NAME);
            commands.trigger(PhaseFinished::<P>(std::marker::PhantomData));
        }
    }

    // find phase nodes that have all their dependencies met and haven't started yet
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

    // trigger StartNode for each ready phase node
    for node in nodes_to_start {
        dag.started_nodes.push(node);

        // trigger targeted
        if let Some(entity) = dag.node_entities.get(&node) {
            commands.entity(*entity).trigger(StartNode);
        }
    }
}

/// Generic cleanup system that clears the temporary state of the dag (completed and started nodes).
///
/// Enables reuse of the loading phase by resetting its state to the original form.
pub fn reset_loading_dag_state<P: LoadingDagPhase>(dag: Option<ResMut<LoadingDag<P>>>) {
    let Some(mut dag) = dag else {
        return;
    };

    dag.started_nodes.clear();
    dag.completed_nodes.clear();

    info!(
        "[{}] Loading DAG state reset. Ready for reuse.",
        P::PHASE_NAME
    );
}

/// System that destroys the dag preventing it from ever being usable again. Only use this for
/// loading dags that will never be used again. Useful for clearing up the resources and memory.
pub fn nuke_loading_dag<P: LoadingDagPhase>(mut commands: Commands, dag: Option<Res<LoadingDag<P>>>) {
    let Some(dag) = dag else {
        return;
    };

    for entity in dag.node_entities.values() {
        commands.entity(*entity).despawn();
    }

    commands.remove_resource::<LoadingDag<P>>();

    info!(
        "[{}] Loading DAG has been nuked and removed from memory.",
        P::PHASE_NAME
    );
}

/// A global polling system that checks if any loading tasks have finished.
///
/// When a loading task finishes, this system applies the tasks' returned commands and despawns
/// the task entity.
pub fn poll_all_loading_tasks(
    mut tasks: Query<(Entity, &mut LoadingTaskComponent)>,
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

/// Returns true if the DAG for the specified phase type is complete.
pub fn loading_dag_is_complete<P: LoadingDagPhase>(dag: Option<Res<LoadingDag<P>>>) -> bool {
    dag.is_none_or(|d| d.completed_nodes.len() == d.dependencies.len())
}

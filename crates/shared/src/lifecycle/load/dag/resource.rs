use super::components::{LoadingDagPhase, StartNode};
use bevy::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;

/// A resource that tracks the dependency graph for a specific loading phase and node type.
///
/// This resource manages which nodes have started and finished, and maps node enums
/// to their dedicated ECS entities for targeted triggers.
#[derive(Resource)]
pub struct LoadingDag<N: LoadingDagPhase> {
    /// Mapping of nodes to their required dependencies.
    pub dependencies: HashMap<N, Vec<N>>,
    /// List of nodes that have already been triggered to start.
    pub started_nodes: Vec<N>,
    /// List of nodes that have successfully completed their tasks.
    pub completed_nodes: Vec<N>,
    /// Mapping of node enums to their dedicated ECS entities for targeted observers.
    pub node_entities: HashMap<N, Entity>,
}

impl<N: LoadingDagPhase> Default for LoadingDag<N> {
    fn default() -> Self {
        Self {
            dependencies: HashMap::new(),
            started_nodes: Vec::new(),
            completed_nodes: Vec::new(),
            node_entities: HashMap::new(),
        }
    }
}

/// A helper struct that wraps the Bevy `App` and carries the Node marker type.
///
/// This allows for a fluent API where the node type is specified once.
pub struct LoadingPhaseProxy<'a, N: LoadingDagPhase> {
    app: &'a mut App,
    _marker: PhantomData<N>,
}

impl<'a, N: LoadingDagPhase> LoadingPhaseProxy<'a, N> {
    /// Adds an asynchronous loading node to this loading phase.
    pub fn add_node<M>(
        &mut self,
        node: N,
        system: impl IntoSystem<On<'static, 'static, StartNode<N>>, (), M> + Send + 'static,
    ) -> &mut Self {
        self.init_node_dag();

        // spawn a dedicated entity for this node
        // we attach the node enum itself as a component so it can be used for filtering
        let node_entity = self.app.world_mut().spawn(node).id();

        // attach the observer globally
        self.app.add_observer(system);

        // register the entity in the DAG
        let mut dag = self.app.world_mut().resource_mut::<LoadingDag<N>>();
        dag.node_entities.insert(node, node_entity);

        // ensure the node exists in dependencies map even if it has no deps yet
        dag.dependencies.entry(node).or_default();

        self
    }

    /// Adds a dependency edge between two loading nodes within this phase.
    ///
    /// The `to` node will wait until the `from` node has completed.
    pub fn add_edge(&mut self, from: N, to: N) -> &mut Self {
        self.init_node_dag();

        let mut dag = self.app.world_mut().resource_mut::<LoadingDag<N>>();
        dag.dependencies.entry(to).or_default().push(from);

        self
    }

    /// Ensures the DAG resource and coordinator are initialized for this node type.
    fn init_node_dag(&mut self) {
        if !self.app.world().contains_resource::<LoadingDag<N>>() {
            self.app.init_resource::<LoadingDag<N>>();
            // register the coordinator that evaluates what to start next
            self.app
                .add_observer(super::systems::evaluate_dag_dependencies::<N>);
        }
    }
}

/// Extension trait for the Bevy App to provide a fluent API for configuring asynchronous loading phases.
pub trait LoadingAppExt {
    /// Enters a loading phase context to register nodes and edges for a specific node enum type.
    fn configure_loading_phase<N: LoadingDagPhase>(&mut self) -> LoadingPhaseProxy<'_, N>;
}

impl LoadingAppExt for App {
    fn configure_loading_phase<N: LoadingDagPhase>(&mut self) -> LoadingPhaseProxy<'_, N> {
        LoadingPhaseProxy {
            app: self,
            _marker: PhantomData,
        }
    }
}

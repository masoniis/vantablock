use super::components::{LoadingDagPhase, StartNode};
use bevy::prelude::*;
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;

/// A resource that tracks the dependency graph for a specific loading phase.
///
/// This resource manages which phase nodes have started and finished, and maps node types
/// to their dedicated ECS entities for targeted triggers.
#[derive(Resource)]
pub struct LoadingDag<P: LoadingDagPhase> {
    /// Mapping of node types to their required dependencies.
    pub dependencies: HashMap<TypeId, Vec<TypeId>>,
    /// List of node types that have already been triggered to start.
    pub started_nodes: Vec<TypeId>,
    /// List of node types that have successfully completed their tasks.
    pub completed_nodes: Vec<TypeId>,
    /// Mapping of node types to their dedicated ECS entities for targeted observers.
    pub node_entities: HashMap<TypeId, Entity>,
    _marker: PhantomData<P>,
}

impl<P: LoadingDagPhase> Default for LoadingDag<P> {
    fn default() -> Self {
        Self {
            dependencies: HashMap::new(),
            started_nodes: Vec::new(),
            completed_nodes: Vec::new(),
            node_entities: HashMap::new(),
            _marker: PhantomData,
        }
    }
}

/// A helper struct that wraps the Bevy `App` and carries the Phase marker type.
///
/// This allows for a fluent API where the phase type is specified once.
pub struct LoadingPhaseProxy<'a, P: LoadingDagPhase> {
    app: &'a mut App,
    _marker: PhantomData<P>,
}

impl<'a, P: LoadingDagPhase> LoadingPhaseProxy<'a, P> {
    /// Adds an asynchronous loading node to this loading phase.
    pub fn add_node<Node: Component + 'static, M>(
        &mut self,
        node: Node,
        system: impl IntoSystem<On<'static, 'static, StartNode>, (), M> + Send + 'static,
    ) -> &mut Self {
        self.init_node_dag();

        let node_type = TypeId::of::<Node>();

        // spawn a dedicated entity for this node
        let node_entity = self.app.world_mut().spawn(node).id();

        // attach the observer specifically to this entity
        self.app.world_mut().entity_mut(node_entity).observe(system);

        // register the entity in the DAG
        let mut dag = self.app.world_mut().resource_mut::<LoadingDag<P>>();
        dag.node_entities.insert(node_type, node_entity);

        // ensure the node exists in dependencies map even if it has no deps yet
        dag.dependencies.entry(node_type).or_default();

        self
    }

    /// Adds a dependency between two loading nodes within this phase.
    ///
    /// The `Dependent` node will wait until the `Dependency` node has completed.
    pub fn add_dependency<Dependent: 'static, Dependency: 'static>(
        &mut self,
        _dependent: Dependent,
        _dependency: Dependency,
    ) -> &mut Self {
        self.init_node_dag();

        let dependent_type = TypeId::of::<Dependent>();
        let dependency_type = TypeId::of::<Dependency>();

        let mut dag = self.app.world_mut().resource_mut::<LoadingDag<P>>();
        dag.dependencies
            .entry(dependent_type)
            .or_default()
            .push(dependency_type);

        self
    }

    /// Ensures the DAG resource and coordinator are initialized for this phase type.
    fn init_node_dag(&mut self) {
        if !self.app.world().contains_resource::<LoadingDag<P>>() {
            self.app.init_resource::<LoadingDag<P>>();
            // register the coordinator that evaluates what to start next
            self.app
                .add_observer(super::systems::evaluate_dag_dependencies::<P>);
        }
    }
}

/// Extension trait for the Bevy App to provide a fluent API for configuring asynchronous loading phases.
pub trait LoadingAppExt {
    /// Enters a loading phase context to register nodes and dependencies for a specific phase marker type.
    fn configure_loading_phase<P: LoadingDagPhase>(&mut self) -> LoadingPhaseProxy<'_, P>;
}

impl LoadingAppExt for App {
    fn configure_loading_phase<P: LoadingDagPhase>(&mut self) -> LoadingPhaseProxy<'_, P> {
        LoadingPhaseProxy {
            app: self,
            _marker: PhantomData,
        }
    }
}

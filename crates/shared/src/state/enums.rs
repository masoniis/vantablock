use bevy::prelude::{Resource, States};

/// A state representing whether the server simulation is currently loading, paused or executing.
#[derive(States, Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SimulationState {
    #[default]
    Loading,
    Running,
    Paused,
}

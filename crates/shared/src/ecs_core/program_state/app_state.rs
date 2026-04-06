use bevy::prelude::{Resource, States};

#[derive(States, Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    StartingUp,
    Running,
    ShuttingDown,
}

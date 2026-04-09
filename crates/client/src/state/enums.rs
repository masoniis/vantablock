use bevy::prelude::{Resource, StateSet, States, SubStates};
use bevy::render::extract_resource::ExtractResource;

/// The state of the overall client app runtime.
///
/// This represents the wrapping program itself, unrelated to game logic.
#[derive(States, Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, ExtractResource)]
pub enum ClientAppState {
    /// The client is loading up essential data. The app loop has not started.
    #[default]
    StartingUp,
    /// The main application loop is active and executing.
    Running,
    /// The application is performing cleanup operations before process termination.
    ShuttingDown,
}

/// A sub-state of `ClientAppState::Running`.
///
/// Represents the game state and lifecycle.
#[derive(SubStates, Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(ClientAppState = ClientAppState::Running)]
pub enum ClientGameState {
    /// The user is navigating the main menu UI.
    #[default]
    MainMenu,
    /// The client is attempting to establish a connection to a server or load into a world.
    Connecting,
    /// The client is actively connected and in a game session.
    Playing,
}

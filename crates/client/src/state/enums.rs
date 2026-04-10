use bevy::prelude::{StateSet, SubStates};
use shared::lifecycle::state::enums::AppState;

/// A sub-state of `ClientAppState::Running`.
///
/// Represents the game state and lifecycle.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Running)]
pub enum ClientGameState {
    /// The user is navigating the main menu UI.
    #[default]
    MainMenu,
    /// The client is attempting to establish a connection to a server or load into a world.
    Connecting,
    /// The client is actively connected and in a game session.
    Playing,
}

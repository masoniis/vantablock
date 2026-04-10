use bevy::prelude::{StateSet, SubStates};
use shared::lifecycle::state::enums::AppState;

/// A sub-state of `ClientAppState::Running`.
///
/// Represents the game state and lifecycle.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Running)]
pub enum ClientGameState {
    /// The client is currently launching and loading data before hitting the main menu.
    #[default]
    Loading,
    /// The user is navigating the main menu UI.
    MainMenu,
    /// The client is attempting to establish a connection to a server.
    Connecting,
    /// A server connection is formed but the client is waiting for initial chunk data
    /// to arrive.
    WorldLoading,
    /// The client is actively connected and in a game session.
    Playing,
    /// The pause menu is open. In single-player, this stops the clock. In multiplayer,
    /// it doesn't.
    Paused,
    /// Disconnect requested, clean up world/meshes, serialization, etc.
    Disconnecting,
    /// An error occurred (kick reason, connection failure)
    Error,
}

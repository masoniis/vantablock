use bevy::prelude::{StateSet, SubStates};
use shared::lifecycle::state::enums::AppState;

/// High-level client state.
/// Sub-state of `AppState::Running`.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Running)]
pub enum ClientState {
    /// Initial asset loading, shader compilation, and warmup.
    #[default]
    Loading,
    /// User is in the main menu.
    MainMenu,
    /// A world session is active (local or remote).
    InGame,
    /// Terminal state for fatal errors/disconnects to clean up the session.
    Error,
}

/// Detailed session lifecycle.
/// Sub-state of `ClientState::InGame`.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(ClientState = ClientState::InGame)]
pub enum InGameState {
    /// Establishing network connection or initializing local server.
    #[default]
    Connecting,
    /// Receiving initial chunks and generating voxel meshes.
    WorldLoading,
    /// Active gameplay.
    Playing,
    /// Logic/Physics paused (single-player).
    Paused,
    /// Tearing down the world, closing sockets, and clearing VRAM.
    Disconnecting,
}

use bevy::{
    prelude::{StateSet, SubStates},
    state::state::States,
};
use shared::lifecycle::state::enums::AppState;

/// A state representing whether the server simulation is currently loading, paused or executing.
// TODO: should no longer need once simulation is fully server-sided
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SimulationState {
    #[default]
    /// The simulation is preparing to start.
    Loading,
    /// The simulation is active and ticking.
    Running,
    /// The simulation is paused.
    Paused,
}

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
    /// Receiving initial chunks and generating block meshes.
    WorldLoading,
    /// Active gameplay.
    Playing,
    /// Logic/Physics paused (single-player).
    Paused,
    /// Tearing down the world, closing sockets, and clearing VRAM.
    Disconnecting,
}

/// Tracks the network topology and authority level of the active session.
/// Sub-state of `ClientState::InGame`.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(ClientState = ClientState::InGame)]
pub enum SessionTopology {
    /// The client is running a local background server (Singleplayer).
    /// The client has Host authority to pause the game, run server commands, etc.
    #[default]
    Internal,

    /// The client is connected to a remote dedicated server (Multiplayer).
    /// The client has standard permissions and cannot pause the simulation.
    External,
}

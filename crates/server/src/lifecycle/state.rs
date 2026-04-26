use bevy::{
    ecs::system::ResMut,
    prelude::{StateSet, SubStates},
    state::state::{NextState, OnEnter},
};
use shared::lifecycle::state::enums::AppState;

/// Detailed lifecycle state of the dedicated or local background server.
///
/// Substate of `AppState::Running`.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Running)]
pub enum ServerState {
    #[default]
    /// The server is loading data (biomes, blocks) and other tasks needed to be
    /// finished before a user connects.
    ///
    /// The network port is **closed** during this phase to prevent clients from
    /// joining an uninitialized world.
    Initializing,

    /// The server is fully booted.
    /// Entering this state binds the UDP socket, starts accepting client connections,
    /// and begins ticking the main game simulation.
    Running,

    /// The server has received a shutdown signal.
    /// New connections are rejected, existing players are gracefully kicked,
    /// and chunk data is safely flushed to the database.
    Terminating,
}

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{
    prelude::{App, Plugin},
    state::app::AppExtStates,
};

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<ServerState>();

        // immediately set to running since no initialize steps at the moment
        app.add_systems(
            OnEnter(ServerState::Initializing),
            |mut server_state: ResMut<NextState<ServerState>>| {
                server_state.set(ServerState::Running);
            },
        );
    }
}

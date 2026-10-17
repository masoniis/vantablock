use crate::lifecycle::state::ServerState;
use bevy::prelude::*;
use shared::lifecycle::state::AppState;

pub struct ServerLoadPlugin;

impl Plugin for ServerLoadPlugin {
    fn build(&self, app: &mut App) {
        // handle transition to running state when server initialization is done
        app.add_systems(
            Update,
            (transition_to_running.run_if(in_state(AppState::Running)),)
                .chain()
                .run_if(in_state(ServerState::Initializing)),
        );
    }
}

fn transition_to_running(mut server_state: ResMut<NextState<ServerState>>) {
    server_state.set(ServerState::Running);
}

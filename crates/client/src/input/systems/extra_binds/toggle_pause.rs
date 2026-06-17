use crate::lifecycle::state::InGameState;
use bevy::prelude::*;

pub fn toggle_pause_system(
    current_state: Res<State<InGameState>>,
    mut next_state: ResMut<NextState<InGameState>>,
) {
    match current_state.get() {
        InGameState::Playing => {
            next_state.set(InGameState::Paused);
        }
        InGameState::Paused => {
            next_state.set(InGameState::Playing);
        }
        _ => {}
    }
}

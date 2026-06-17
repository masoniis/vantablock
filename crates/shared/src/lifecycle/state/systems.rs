use bevy::{prelude::*, state::state::FreelyMutableState};

/// A generic system that unconditionally transitions to the target state.
pub fn transition_to<S: States + Copy + FreelyMutableState>(
    target_state: S,
) -> impl FnMut(ResMut<NextState<S>>) {
    move |mut next_state| {
        next_state.set(target_state);
    }
}

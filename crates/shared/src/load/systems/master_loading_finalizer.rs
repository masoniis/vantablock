use crate::{
    load::{LoadingTracker, OnLoadComplete},
    prelude::*,
};
use bevy::ecs::prelude::*;
use bevy::prelude::{NextState, States};
use bevy::state::state::FreelyMutableState;

/// The master system that runs in the simulation world. It checks the shared
/// LoadingTracker and the OnLoadComplete resource to make the final decision
/// on when to transition the app's state.
#[instrument(skip_all)]
pub fn master_finalize_loading_system<T: States + FreelyMutableState + Copy>(
    // Input
    loading_tracker: Option<Res<LoadingTracker>>,
    on_complete: Option<Res<OnLoadComplete<T>>>,

    // Output (set the next state)
    mut next_state: ResMut<NextState<T>>,
) {
    let Some(loading_tracker) = loading_tracker else {
        return;
    };

    // if we have both the tracker and the "what to do next" instruction
    if let Some(on_complete) = on_complete
        && loading_tracker.is_ready()
    {
        info!(
            "Loading is done. Transitioning to {:?}.",
            on_complete.destination
        );

        // set the next state
        next_state.set(on_complete.destination);
    }
}

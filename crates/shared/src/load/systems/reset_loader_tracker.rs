use crate::load::LoadingTracker;
use crate::prelude::*;
use bevy::ecs::prelude::*;

#[instrument(skip_all)]
pub fn reset_loading_tracker_system(loading_tracker: Option<Res<LoadingTracker>>) {
    let Some(loading_tracker) = loading_tracker else {
        return;
    };

    info!("Resetting loading tracker...");
    loading_tracker.reset();
}

use crate::network::InboundClockSyncMessage;
use bevy::prelude::*;
use shared::time::WorldClockResource;
use std::time::Duration;

/// Adopts the server's authoritative world clock.
///
/// Between syncs the client advances its own copy every frame, so this only corrects drift (and
/// applies time jumps). The newest sync wins.
pub fn apply_clock_sync_system(
    mut syncs: MessageReader<InboundClockSyncMessage>,
    mut clock: ResMut<WorldClockResource>,
) {
    let Some(sync) = syncs.read().last() else {
        return;
    };
    clock.elapsed = Duration::from_secs_f64(sync.elapsed_seconds.max(0.0));
    clock.day_cycle_paused = sync.day_cycle_paused;
    let day = clock.day_duration.as_secs_f32();
    clock.time_of_day = Duration::from_secs_f32(sync.time_of_day_seconds.rem_euclid(day));
}

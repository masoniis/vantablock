use crate::network::InboundSyncTimeMessage;
use bevy::prelude::*;
use common::time::WorldClockResource;
use std::time::Duration;

/// Snaps the client's world clock to the server's. Between syncs the client's clock keeps
/// ticking on its own, so the sky moves smoothly.
pub fn follow_server_clock_system(
    mut syncs: MessageReader<InboundSyncTimeMessage>,
    mut world_clock: ResMut<WorldClockResource>,
) {
    if let Some(sync) = syncs.read().last() {
        world_clock.total_days = sync.total_days;
        world_clock.time_of_day = Duration::from_secs_f32(sync.time_of_day);
        world_clock.paused = sync.paused;
    }
}

use crate::network::types::ClockSyncTimer;
use bevy::prelude::*;
use common::{
    network::{ClockSync, ServerMessage},
    time::WorldClockResource,
};
use lightyear::{prelude::server::*, prelude::*};
use tracing::error;

/// Periodically broadcasts the authoritative world clock so every client animates and lights
/// the world in step.
pub fn send_clock_sync(
    mut timer: ResMut<ClockSyncTimer>,
    time: Res<Time>,
    clock: Res<WorldClockResource>,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Server>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished()
        && let Some(server) = server
    {
        let message = ServerMessage::SyncWorldClock {
            elapsed_seconds: clock.elapsed.as_secs_f64(),
            time_of_day_seconds: clock.time_of_day.as_secs_f32(),
            day_cycle_paused: clock.day_cycle_paused,
        };
        if let Err(e) =
            sender.send::<_, ClockSync>(&message, server.into_inner(), &NetworkTarget::All)
        {
            error!("Failed to send clock sync: {:?}", e);
        }
    }
}

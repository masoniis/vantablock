use crate::{network::types::MessageTimer, world::clock::ClockAdjustedMessage};
use bevy::prelude::*;
use common::{
    network::{ChatAndSystem, ServerMessage},
    time::WorldClockResource,
};
use lightyear::{prelude::server::*, prelude::*};
use tracing::{debug, error};

/// Sends the world clock to every client once a second, and right away after it was changed.
pub fn send_sync_time(
    mut timer: ResMut<MessageTimer>,
    mut adjusted: MessageReader<ClockAdjustedMessage>,
    time: Res<Time>,
    world_clock: Res<WorldClockResource>,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Endpoint, With<Server>>>,
) {
    timer.0.tick(time.delta());
    let changed = adjusted.read().count() > 0;
    if !(timer.0.just_finished() || changed) {
        return;
    }
    let Some(server) = server else {
        return;
    };

    let message = ServerMessage::SyncTime {
        total_days: world_clock.total_days,
        time_of_day: world_clock.time_of_day.as_secs_f32(),
        paused: world_clock.paused,
    };
    debug!("Sending SyncTime message: {:?}", message);
    if let Err(e) =
        sender.send::<_, ChatAndSystem>(&message, server.into_inner(), &NetworkTarget::All)
    {
        error!("Failed to send SyncTime message: {:?}", e);
    }
}

use crate::network::receive::ecs_messages::{InboundTimeJumpMessage, InboundToggleDayCycleMessage};
use bevy::prelude::*;
use common::time::WorldClockResource;

/// Jumps the authoritative time of day when a client asks; the next clock sync carries it out.
pub fn handle_time_jump_requests(
    mut requests: MessageReader<InboundTimeJumpMessage>,
    mut clock: ResMut<WorldClockResource>,
) {
    for request in requests.read() {
        clock.shift_time_of_day(request.seconds);
        info!(
            "Client {:?} jumped the time of day by {}s",
            request.client, request.seconds
        );
    }
}

pub fn handle_day_cycle_toggle_requests(
    mut requests: MessageReader<InboundToggleDayCycleMessage>,
    mut clock: ResMut<WorldClockResource>,
) {
    for request in requests.read() {
        clock.day_cycle_paused = !clock.day_cycle_paused;
        info!(
            "Client {:?} {} the day cycle",
            request.client,
            if clock.day_cycle_paused {
                "paused"
            } else {
                "resumed"
            }
        );
    }
}

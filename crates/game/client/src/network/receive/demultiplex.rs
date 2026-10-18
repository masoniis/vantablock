use crate::network::InboundClockSyncMessage;
use bevy::{ecs::message::MessageWriter, prelude::*};
use common::network::protocol::ServerMessage;
use lightyear::prelude::MessageReceiver;

/// Network demultiplexer, translates a `ServerMessage` into a local ecs message
/// to be consumed directly by other systems.
///
/// While it produces a decent amount of boiler plate since we need a client
/// ECS event for each enum variant of the incoming server message, this is
/// worth for efficiency (iterating demuiltiplexed data).
pub fn translate_server_network_messages(
    // incoming network messages
    mut query: Query<&mut MessageReceiver<ServerMessage>>,
    // outgoing ECS messages
    mut ev_clock: MessageWriter<InboundClockSyncMessage>,
) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
            match message {
                ServerMessage::SyncWorldClock {
                    elapsed_seconds,
                    time_of_day_seconds,
                    day_cycle_paused,
                } => {
                    ev_clock.write(InboundClockSyncMessage {
                        elapsed_seconds,
                        time_of_day_seconds,
                        day_cycle_paused,
                    });
                }
            }
        }
    }
}

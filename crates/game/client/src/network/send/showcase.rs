use super::ecs_messages::{OutboundSetTimeOfDayMessage, OutboundTeleportMessage};
use bevy::ecs::prelude::*;
use common::network::protocol::{ChatAndSystem, ClientMessage};
use lightyear::prelude::{Client, MessageSender};

/// Forwards the showcase's teleport and time-of-day requests to the server. Without a
/// connection they are dropped.
pub fn send_showcase_messages_system(
    mut teleports: MessageReader<OutboundTeleportMessage>,
    mut times: MessageReader<OutboundSetTimeOfDayMessage>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<Client>>,
) {
    let Ok(mut sender) = sender_query.single_mut() else {
        teleports.clear();
        times.clear();
        return;
    };

    for message in times.read() {
        sender.send::<ChatAndSystem>(ClientMessage::SetTimeOfDay {
            fraction: message.fraction,
        });
    }

    for message in teleports.read() {
        sender.send::<ChatAndSystem>(ClientMessage::Teleport {
            position: message.position,
        });
    }
}

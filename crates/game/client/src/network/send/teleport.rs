use super::ecs_messages::OutboundTeleportMessage;
use bevy::ecs::prelude::*;
use common::network::protocol::{ChatAndSystem, ClientMessage};
use lightyear::prelude::{Client, MessageSender};

/// Forwards the local player's teleport requests to the server. Without a connection they are
/// dropped.
pub fn send_teleport_messages_system(
    mut teleports: MessageReader<OutboundTeleportMessage>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<Client>>,
) {
    let Ok(mut sender) = sender_query.single_mut() else {
        teleports.clear();
        return;
    };

    for message in teleports.read() {
        sender.send::<ChatAndSystem>(ClientMessage::Teleport {
            position: message.position,
        });
    }
}

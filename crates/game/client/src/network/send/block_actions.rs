use super::ecs_messages::{OutboundBreakBlockMessage, OutboundPlaceBlockMessage};
use bevy::ecs::prelude::*;
use common::network::protocol::{BlockUpdates, ClientMessage};
use lightyear::prelude::{Client, MessageSender};

/// Forwards the local player's break/place intents to the server as `ClientMessage`s.
///
/// The player systems have already applied the edit locally (client-side prediction), so
/// this only reports the intent. Without a connection the intents are dropped.
pub fn send_block_action_messages_system(
    mut breaks: MessageReader<OutboundBreakBlockMessage>,
    mut places: MessageReader<OutboundPlaceBlockMessage>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<Client>>,
) {
    let Ok(mut sender) = sender_query.single_mut() else {
        breaks.clear();
        places.clear();
        return;
    };

    for message in breaks.read() {
        sender.send::<BlockUpdates>(ClientMessage::BreakBlock {
            position: message.position,
        });
    }

    for message in places.read() {
        sender.send::<BlockUpdates>(ClientMessage::PlaceBlock {
            position: message.position,
            block_id: message.block_id,
        });
    }
}

use crate::network::receive::ecs_messages::*;
use bevy::{ecs::message::MessageWriter, prelude::*};
use lightyear::prelude::MessageReceiver;
use shared::network::protocol::ClientMessage;

/// Network demultiplexer, translates a `ClientMessage` into a local ecs message
///
/// This system listens for incoming client messages and dispatches a corresponding
/// ECS message for each enum variant of the incoming client message. This is
/// necessary to allow multiple systems to react to specific client messages.
pub fn translate_client_network_messages(
    // incoming network messages
    mut query: Query<(Entity, &mut MessageReceiver<ClientMessage>)>,
    // outgoing ECS messages
    mut ev_time_jump: MessageWriter<InboundTimeJumpMessage>,
    mut ev_toggle_day_cycle: MessageWriter<InboundToggleDayCycleMessage>,
) {
    for (client, mut receiver) in query.iter_mut() {
        for message in receiver.receive() {
            match message {
                ClientMessage::RequestTimeJump { seconds } => {
                    ev_time_jump.write(InboundTimeJumpMessage { client, seconds });
                }
                ClientMessage::RequestToggleDayCycle => {
                    ev_toggle_day_cycle.write(InboundToggleDayCycleMessage { client });
                }
            }
        }
    }
}

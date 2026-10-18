//! Defines local ECS messages for demultiplexed network messages received from clients.
//! These messages are dispatched by the network demultiplexer to allow multiple systems
//! to react to specific client messages without all of them needing to read from the
//! `MessageReceiver<ClientMessage>` directly.

use bevy::ecs::message::Message;
use bevy::prelude::*;

#[derive(Message, Clone)]
pub struct InboundTimeJumpMessage {
    /// The client connection entity that asked.
    pub client: Entity,
    pub seconds: f32,
}

#[derive(Message, Clone)]
pub struct InboundToggleDayCycleMessage {
    pub client: Entity,
}

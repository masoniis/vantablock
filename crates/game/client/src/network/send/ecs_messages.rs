//! Defines the local ecs messages that request a network send
//!
//! These mirror the `receive` module's ecs messages in the other direction: gameplay code
//! writes one of these and a system in this module turns it into the real network message.

use bevy::{ecs::message::Message, prelude::*};

/// The local player wants the server to break a block.
#[derive(Message)]
pub struct OutboundBreakBlockMessage {
    pub position: IVec3,
}

/// The local player wants the server to place a block.
#[derive(Message)]
pub struct OutboundPlaceBlockMessage {
    pub position: IVec3,
    pub block_id: u8,
}

/// The local player wants the server to move it somewhere.
#[derive(Message)]
pub struct OutboundTeleportMessage {
    pub position: Vec3,
}

/// The local player wants the server to set the time of day (a fraction of the day).
#[derive(Message)]
pub struct OutboundSetTimeOfDayMessage {
    pub fraction: f32,
}

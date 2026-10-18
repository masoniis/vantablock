//! Defines the local ecs messages for demultiplexed network messages
//!
//! For the most part, these messages are simply duplicated structurs
//! of the original network messages our client receives.

use bevy::ecs::message::Message;

#[derive(Message, Clone, Copy, Debug)]
pub struct InboundClockSyncMessage {
    pub elapsed_seconds: f64,
    pub time_of_day_seconds: f32,
    pub day_cycle_paused: bool,
}

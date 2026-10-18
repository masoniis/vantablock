use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// A protocol defining all the direct client messages that a client communicates
/// to the server. Note that this only defines the boundaries for message comms
/// while the client also has other means of communication like replication.
#[derive(Message, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ClientMessage {
    /// Request the server to jump the time of day by `seconds` (negative to rewind).
    RequestTimeJump { seconds: f32 },
    /// Request the server to pause or resume the day/night cycle.
    RequestToggleDayCycle,
}

/// A protocol defining all the direct server messages that a server communicates
/// to its clients. Note that this only defines the boundaries for message comms
/// while the server also has other means of communication like replication.
#[derive(Message, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ServerMessage {
    /// The authoritative world clock: animation time and time of day.
    SyncWorldClock {
        elapsed_seconds: f64,
        time_of_day_seconds: f32,
        day_cycle_paused: bool,
    },
}

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

pub struct NetMessagesPlugin;

impl Plugin for NetMessagesPlugin {
    fn build(&self, app: &mut App) {
        app.register_message::<ClientMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<ServerMessage>()
            .add_direction(NetworkDirection::ServerToClient);
    }
}

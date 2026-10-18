use bevy::prelude::*;
use lightyear::prelude::{
    AppChannelExt, ChannelMode, ChannelSettings, NetworkDirection, ReliableSettings,
};

/// Periodic authoritative world clock updates.
///
/// **Mode:** `SequencedUnreliable`
/// - Unreliable because a newer sync is always on its way.
/// - Sequenced so a late, older sync never rewinds the clock.
///
/// **Direction:** `ServerToClient`
pub struct ClockSync;

/// Text chat, commands, and critical server system broadcasts.
///
/// **Mode:** `OrderedReliable`
/// - Reliable to ensure messages are not lost.
/// - Ordered so that conversations and requests resolve in the order they were made.
///
/// **Direction:** `Bidirectional`
pub struct ChatAndSystem;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

/// This plugin defines the network routing "pipes" (channels) that messages are sent down.
/// Each channel is configured with specific delivery guarantees suited for its data type.
pub struct NetChannelsPlugin;

impl Plugin for NetChannelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_channel::<ClockSync>(ChannelSettings {
            mode: ChannelMode::SequencedUnreliable,
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        app.add_channel::<ChatAndSystem>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        })
        .add_direction(NetworkDirection::Bidirectional);
    }
}

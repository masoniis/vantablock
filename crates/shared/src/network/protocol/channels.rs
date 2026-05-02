use bevy::prelude::*;
use lightyear::prelude::{
    AppChannelExt, ChannelMode, ChannelSettings, NetworkDirection, ReliableSettings,
};

/// High-frequency input stream for player movement and orientation.
///
/// **Mode:** `SequencedUnreliable`
/// - Unreliable because dropped packets don't matter as a newer one is coming next tick.
/// - Sequenced because older out-of-order packets should be discarded to prevent "jitter".
///
/// **Direction:** `ClientToServer`
pub struct PlayerMovement;

/// Requests for block destruction/placement and the resulting state updates.
///
/// **Mode:** `OrderedReliable`
/// - Reliable because a dropped packet would leave a "ghost block" (desync).
/// - Ordered because mining block A then placing block B in the same spot must resolve in that exact sequence.
///
/// **Direction:** `Bidirectional` (Requests C->S, Updates S->C)
pub struct BlockUpdates;

/// Massive terrain payloads for world generation and chunk loading.
///
/// **Mode:** `UnorderedReliable`
/// - Reliable because missing chunk data leaves a hole in the world.
/// - Unordered because Chunk A arriving before Chunk B doesn't break simulation and prevents head-of-line blocking.
///
/// **Direction:** `ServerToClient`
pub struct ChunkData;

/// Spawning and despawning of players, dropped items, or mobs.
///
/// **Mode:** `OrderedReliable`
/// - Reliable because ghost entities or missing spawns break the game state.
/// - Ordered to ensure spawn events precede any updates for that entity.
///
/// **Direction:** `ServerToClient`
pub struct EntityLifecycle;

/// Text chat and critical server system broadcasts.
///
/// **Mode:** `OrderedReliable`
/// - Reliable to ensure messages are not lost.
/// - Ordered so that conversations and system logs make sense.
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
        app.add_channel::<PlayerMovement>(ChannelSettings {
            mode: ChannelMode::SequencedUnreliable,
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);

        app.add_channel::<BlockUpdates>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        })
        .add_direction(NetworkDirection::Bidirectional);

        app.add_channel::<ChunkData>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings { ..default() }),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        app.add_channel::<EntityLifecycle>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
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

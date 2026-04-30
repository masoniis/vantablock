pub struct PlayerMovement;
pub struct BlockUpdates;
pub struct ChunkData;
pub struct EntityLifecycle;
pub struct ModStateSync;
pub struct ChatAndSystem;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::{
    AppChannelExt, ChannelMode, ChannelSettings, NetworkDirection, ReliableSettings,
};

pub struct NetChannelsPlugin;

/// A plugin that defines the shared client-server networking protocols
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

        app.add_channel::<ModStateSync>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        })
        .add_direction(NetworkDirection::Bidirectional);

        app.add_channel::<ChatAndSystem>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        })
        .add_direction(NetworkDirection::Bidirectional);
    }
}

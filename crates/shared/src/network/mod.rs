pub mod channel;
pub mod protocol;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use channel::{
    BlockUpdates, ChatAndSystem, ChunkData, EntityLifecycle, ModStateSync, PlayerMovement,
};
use lightyear::prelude::{AppChannelExt, ChannelMode, ChannelSettings, ReliableSettings};
use protocol::NetworkProtoclPlugin;

pub struct NetworkPlugin;

/// A plugin that defines sets up the shared network stuff
impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkProtoclPlugin);

        app.add_channel::<PlayerMovement>(ChannelSettings {
            mode: ChannelMode::SequencedUnreliable,
            ..default()
        });

        app.add_channel::<BlockUpdates>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        });

        app.add_channel::<ChunkData>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings { ..default() }),
            ..default()
        });

        app.add_channel::<EntityLifecycle>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        });

        app.add_channel::<ModStateSync>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        });

        app.add_channel::<ChatAndSystem>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings { ..default() }),
            ..default()
        });
    }
}

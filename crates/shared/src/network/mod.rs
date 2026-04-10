pub mod channel;
pub mod protocol;
pub mod state;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use channel::{
    BlockUpdates, ChatAndSystem, ChunkData, EntityLifecycle, ModStateSync, PlayerMovement,
};
use lightyear::prelude::{AppChannelExt, ChannelMode, ChannelSettings, ReliableSettings};
use protocol::NetworkProtoclPlugin;

use crate::network::state::NetworkingMode;

pub struct NetworkPlugin;

/// A plugin that defines sets up the shared network stuff
impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkProtoclPlugin);

        // add states
        app.init_state::<NetworkingMode>();

        // add channels
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

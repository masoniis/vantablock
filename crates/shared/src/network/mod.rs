pub mod channel;
pub mod protocol;
pub mod state;

pub const NETWORK_TICK_DURATION: f64 = 1.0 / 60.0;
pub const NETWORK_DEFAULT_PORT: u16 = 5000;

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

pub struct SharedNetworkPlugin;

/// A plugin that defines sets up the shared network stuff
impl Plugin for SharedNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkProtoclPlugin);

        // add states
        app.add_sub_state::<NetworkingMode>();
    }

    fn finish(&self, app: &mut App) {
        // Since the protocol must be added after the lightyear `ClientPlugins` we do lightyear
        // channel registration in the finish method, not the build method.
        // https://docs.rs/lightyear/0.26.4/lightyear/prelude/client/struct.ClientPlugins.html

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

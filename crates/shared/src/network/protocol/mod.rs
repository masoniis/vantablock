pub mod channels;
pub mod client;
pub mod server;

pub use channels::*;
pub use client::ClientMessage;
pub use server::ServerMessage;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::player::components::PlayerLook;
use bevy::prelude::*;
use lightyear::prelude::{
    AppChannelExt, AppComponentExt, AppMessageExt, ChannelMode, ChannelSettings, NetworkDirection,
    ReliableSettings,
};

pub(crate) struct NetworkProtocolPlugin;

/// A plugin that defines the shared client-server networking protocols
impl Plugin for NetworkProtocolPlugin {
    fn build(&self, app: &mut App) {
        // the protocol must be added after the lightyear `ClientPlugins`
        // https://docs.rs/lightyear/0.26.4/lightyear/prelude/client/struct.ClientPlugins.html
        app.register_message::<ClientMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<ServerMessage>()
            .add_direction(NetworkDirection::ServerToClient);

        // register components
        app.register_component::<PlayerLook>();

        // add channels
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

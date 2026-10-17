pub mod channels;
pub mod components;
pub mod messages;

pub use channels::*;
pub use components::NetComponentsPlugin;
pub use messages::{ClientMessage, NetMessagesPlugin, ServerMessage};

// INFO: ---------------------------------
//         plugin group definition
// ---------------------------------------

use bevy::{app::PluginGroupBuilder, prelude::*};

pub struct NetworkProtocolPlugins;

/// A plugin group that defines the shared client-server networking protocols.
///
/// This includes channels, messages, and networked components.
///
/// WARN: Because this group registers the lightyear protocol, it must be
/// added AFTER the lightyear plugin groups for client/server.
/// - See: https://docs.rs/lightyear/0.26.4/lightyear/prelude/client/struct.ClientPlugins.html
impl PluginGroup for NetworkProtocolPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(NetChannelsPlugin)
            .add(NetMessagesPlugin)
            .add(NetComponentsPlugin)
    }
}

pub mod client;
pub mod server;

pub use client::ClientMessage;
pub use server::ServerMessage;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::{AppMessageExt, NetworkDirection};

pub struct NetworkProtoclPlugin;

/// A plugin that defines the shared client-server networking protocols
impl Plugin for NetworkProtoclPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        // Since the protocol must be added after the lightyear `ClientPlugins` we do lightyear
        // protocl registration in the finish method, not the build method.
        // https://docs.rs/lightyear/0.26.4/lightyear/prelude/client/struct.ClientPlugins.html
        app.register_message::<ClientMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<ServerMessage>()
            .add_direction(NetworkDirection::ServerToClient);
    }
}

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
    fn build(&self, app: &mut App) {
        app.register_message::<ClientMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<ServerMessage>()
            .add_direction(NetworkDirection::ServerToClient);
    }
}

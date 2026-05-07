pub mod consts;
pub mod protocol;

pub use consts::*;
pub use protocol::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;

pub struct SharedNetworkPlugin;

/// A plugin that defines sets up the shared network stuff
impl Plugin for SharedNetworkPlugin {
    fn build(&self, app: &mut App) {
        // the protocol must be added after the lightyear `ClientPlugins`
        // https://docs.rs/lightyear/0.26.4/lightyear/prelude/client/struct.ClientPlugins.html
        app.add_plugins(NetworkProtocolPlugins);
    }
}

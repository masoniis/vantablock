pub mod protocol;

pub use protocol::*;

pub const NETWORK_TICK_DURATION: f64 = 1.0 / 60.0;
pub const NETWORK_DEFAULT_PORT: u16 = 5000;

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
        app.add_plugins(NetworkProtocolPlugin);
    }
}

pub mod connection;
pub mod graphics;
pub mod receive;
pub mod send;

pub use receive::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::{app::PluginGroupBuilder, prelude::*};
use lightyear::prelude::client as lightyear_client;
use shared::network::{NETWORK_TICK_DURATION, SharedNetworkPlugin};
use std::time::Duration;

pub struct ClientNetworkPlugins;

impl PluginGroup for ClientNetworkPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // lightyear plugin group and interpolation
            .add_group(lightyear_client::ClientPlugins {
                tick_duration: Duration::from_secs_f64(NETWORK_TICK_DURATION),
            })
            .add(SharedNetworkPlugin)
            // client specific networking plugins
            .add(connection::NetworkConnectionPlugin)
            .add(graphics::NetworkGraphicsPlugin)
            .add(receive::NetworkReceivePlugin)
            .add(send::NetworkSendPlugin)
    }
}

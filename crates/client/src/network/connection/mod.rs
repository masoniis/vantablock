pub mod perform_connection;
pub mod resources;

pub use perform_connection::setup_client;
pub use resources::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin};

pub struct ClientConnectionPlugin;

impl Plugin for ClientConnectionPlugin {
    fn build(&self, app: &mut App) {
        // Triggered manually from UI, which then causes the game state connection state
        app.add_observer(setup_client);

        app.init_resource::<ConnectionSettings>();
    }
}

mod sync_clock;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct ServerEgressPlugin;

impl Plugin for ServerEgressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_clock::send_clock_sync);
    }
}

mod sync_time;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct ServerEgressPlugin;

impl Plugin for ServerEgressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_time::send_sync_time);
    }
}

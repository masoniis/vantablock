mod initiate;
pub mod resources;
mod session;

pub use resources::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub(crate) struct NetworkConnectionPlugin;

impl Plugin for NetworkConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(initiate::on_initiate_connection);

        app.add_systems(Update, session::handle_disconnections)
            .add_observer(session::handle_connections);
    }
}

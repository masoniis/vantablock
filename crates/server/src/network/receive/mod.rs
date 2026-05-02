pub mod block_actions;
pub mod client_message;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct ServerIngressPlugin;

impl Plugin for ServerIngressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                client_message::receive_client_messages,
                block_actions::handle_client_block_requests,
            ),
        );
    }
}

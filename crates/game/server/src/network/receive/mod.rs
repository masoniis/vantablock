pub mod demultiplex;
pub mod ecs_messages;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};
use ecs_messages::*;

pub struct ServerIngressPlugin;

impl Plugin for ServerIngressPlugin {
    fn build(&self, app: &mut App) {
        // register local ecs messages
        app.add_message::<InboundTimeJumpMessage>()
            .add_message::<InboundToggleDayCycleMessage>();

        app.add_systems(Update, demultiplex::translate_client_network_messages);
    }
}

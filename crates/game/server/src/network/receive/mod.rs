pub mod block_actions;
pub mod demultiplex;
pub mod ecs_messages;
pub mod teleport;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};
use bevy::prelude::*;
use ecs_messages::*;

pub struct ServerIngressPlugin;

impl Plugin for ServerIngressPlugin {
    fn build(&self, app: &mut App) {
        // register local ecs messages
        app.add_message::<InboundRequestChunkMessage>()
            .add_message::<InboundBreakBlockMessage>()
            .add_message::<InboundPlaceBlockMessage>()
            .add_message::<InboundTeleportMessage>();

        app.add_systems(
            Update,
            (
                demultiplex::translate_client_network_messages,
                (
                    block_actions::handle_client_block_requests,
                    teleport::handle_teleport_requests,
                ),
            )
                .chain(),
        );
    }
}

//! Everything the client sends to the server.
//!
//! Gameplay code does not touch lightyear's `MessageSender` directly. It writes an
//! `Outbound*Message` ecs message (see `ecs_messages`) and a system here forwards it, which
//! mirrors how `receive` turns incoming network messages into `Inbound*Message`s.
//!
//! The one exception is `RequestChunks`, sent by
//! `world::chunk::loading::manage_distance_based_chunk_loading_targets_system`. That
//! system marks chunks as awaiting data in the same step, so it must know the request really
//! went out (it bails without a connection); a queued message could be dropped and leave
//! permanent holes.

mod block_actions;
mod ecs_messages;

pub use ecs_messages::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};

pub struct NetworkSendPlugin;

impl Plugin for NetworkSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OutboundBreakBlockMessage>();
        app.add_message::<OutboundPlaceBlockMessage>();

        app.add_systems(Update, send_block_action_messages_system);
    }
}

pub use block_actions::send_block_action_messages_system;

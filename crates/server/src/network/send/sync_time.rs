use crate::network::types::MessageTimer;
use bevy::prelude::*;
use lightyear::{prelude::server::*, prelude::*};
use shared::network::{ChatAndSystem, ServerMessage};
use tracing::{error, info};

/// Sends the periodic sync timer to all clients
pub fn send_sync_time(
    mut timer: ResMut<MessageTimer>,
    time: Res<Time>,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Server>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished()
        && let Some(server) = server
    {
        let message = ServerMessage::SyncTime {
            game_time: time.elapsed_secs(),
            tick: 0, // placeholder
        };
        info!("Sending periodic SyncTime message: {:?}", message);
        if let Err(e) =
            sender.send::<_, ChatAndSystem>(&message, server.into_inner(), &NetworkTarget::All)
        {
            error!("Failed to send SyncTime message: {:?}", e);
        }
    }
}

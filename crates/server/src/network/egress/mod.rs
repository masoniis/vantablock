use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::network::{ChatAndSystem, ServerMessage};
use tracing::{error, info};

use super::types::MessageTimer;

pub struct ServerEgressPlugin;

impl Plugin for ServerEgressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, send_sync_time);
    }
}

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

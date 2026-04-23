// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use lightyear::prelude::MessageReceiver;
use shared::network::protocol::server::ServerMessage;
use shared::simulation::chunk::{ChunkBlocksComponent, ChunkLod, ChunkStateManager};

pub struct ClientMessageHandlerPlugin;

impl Plugin for ClientMessageHandlerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_server_messages);
    }
}

fn handle_server_messages(
    mut commands: Commands,
    mut query: Query<&mut MessageReceiver<ServerMessage>>,
    mut chunk_manager: ResMut<ChunkStateManager>,
) {
    for mut receiver in query.iter_mut() {
        for message in receiver.receive() {
            match message {
                ServerMessage::Welcome {
                    entity_id: _,
                    spawn_pos,
                } => {
                    info!("Welcome message received! Spawn pos: {:?}", spawn_pos);
                }
                ServerMessage::ChunkData { coord, data } => {
                    trace!("Received chunk data for {:?}", coord);
                    if !chunk_manager.is_chunk_present_or_loading(coord.pos) {
                        let blocks = ChunkBlocksComponent::from_vec(ChunkLod(0), data);
                        let ent = commands.spawn((blocks, coord.clone())).id();
                        chunk_manager.mark_as_data_ready(coord.pos, ent);
                    }
                }
                _ => {
                    warn!("Unhandled message received");
                }
            }
        }
    }
}

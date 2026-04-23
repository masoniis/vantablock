use crate::network::systems::{ClientConnection, SentWelcome};
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::network::channel::{ChatAndSystem, ChunkData as ChunkChannel};
use shared::network::protocol::server::ServerMessage;
use shared::simulation::chunk::{
    ChunkBlocksComponent, ChunkCoord, ChunkLod, ChunkStateManager, LOAD_DISTANCE, NeedsGenerating,
    WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
};
use std::collections::HashSet;

/// Tracks which chunks have been sent to a specific client.
#[derive(Component, Default)]
pub struct ClientChunkTracker {
    pub sent_chunks: HashSet<IVec3>,
}

pub fn send_welcome_system(
    mut commands: Commands,
    mut player_query: Query<(Entity, &ClientConnection, &Transform), Without<SentWelcome>>,
    mut sender_query: Query<&mut MessageSender<ServerMessage>>,
) {
    for (player_ent, connection, transform) in player_query.iter_mut() {
        if let Ok(mut sender) = sender_query.get_mut(connection.client_entity) {
            debug!("Sending welcome to {:?}", connection.client_entity);
            sender.send::<ChatAndSystem>(ServerMessage::Welcome {
                entity_id: player_ent,
                spawn_pos: transform.translation,
            });
            commands.entity(player_ent).insert(SentWelcome);
        }
    }
}

/// Determines which chunks each player needs and starts loading them.
pub fn manage_player_chunk_loading_system(
    // Input
    player_query: Query<(&Transform, Entity), With<ClientConnection>>,

    // Output
    mut chunk_manager: ResMut<ChunkStateManager>,
    mut commands: Commands,
) {
    for (transform, _player_ent) in player_query.iter() {
        let player_pos = transform.translation;
        let player_chunk_pos = ChunkCoord::world_to_chunk_pos(player_pos);

        // determine desired chunks
        for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
            for z in -LOAD_DISTANCE..=LOAD_DISTANCE {
                for x in -LOAD_DISTANCE..=LOAD_DISTANCE {
                    let coord = IVec3::new(player_chunk_pos.x + x, y, player_chunk_pos.z + z);

                    if !chunk_manager.is_chunk_present_or_loading(coord) {
                        debug!(target:"server_chunk_loading","Server: Marking chunk needs-generation at {:?}", coord);
                        let ent = commands
                            .spawn((
                                NeedsGenerating { lod: ChunkLod(0) },
                                ChunkCoord { pos: coord },
                            ))
                            .id();
                        chunk_manager.mark_as_needs_generating(coord, ent);
                    }
                }
            }
        }
    }
}

/// Sends generated chunk data to clients that need it.
pub fn sync_chunk_data_to_clients_system(
    // Input
    mut client_query: Query<(&Transform, &ClientConnection, &mut ClientChunkTracker)>,
    chunk_query: Query<(&ChunkCoord, &ChunkBlocksComponent)>,
    chunk_manager: Res<ChunkStateManager>,
    mut sender_query: Query<&mut MessageSender<ServerMessage>>,
) {
    for (transform, connection, mut tracker) in client_query.iter_mut() {
        let player_pos = transform.translation;
        let player_chunk_pos = ChunkCoord::world_to_chunk_pos(player_pos);

        let Ok(mut sender) = sender_query.get_mut(connection.client_entity) else {
            continue;
        };

        // find chunks within load distance that haven't been sent yet
        for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
            for z in -LOAD_DISTANCE..=LOAD_DISTANCE {
                for x in -LOAD_DISTANCE..=LOAD_DISTANCE {
                    let coord = IVec3::new(player_chunk_pos.x + x, y, player_chunk_pos.z + z);

                    if tracker.sent_chunks.contains(&coord) {
                        continue;
                    }

                    // check if chunk is generated
                    if let Some(state) = chunk_manager.get_state(coord)
                        && state.is_generated()
                        && let Some(chunk_ent) = chunk_manager.get_entity(coord)
                        && let Ok((_coord, blocks)) = chunk_query.get(chunk_ent)
                    {
                        let data = extract_block_data(blocks);

                        debug!(target:"server_chunk_loading", "Sending chunk {:?} to client {:?}", coord, connection.client_entity);
                        sender.send::<ChunkChannel>(ServerMessage::ChunkData {
                            coord: ChunkCoord { pos: coord },
                            data,
                        });

                        tracker.sent_chunks.insert(coord);
                    }
                }
            }
        }
    }
}

fn extract_block_data(blocks: &ChunkBlocksComponent) -> Vec<u8> {
    let view = blocks.get_view();
    let size = blocks.size();
    let mut data = Vec::with_capacity(size * size * size);

    match view {
        shared::simulation::chunk::ChunkView::Uniform(block_id) => {
            data.resize(size * size * size, block_id);
        }
        shared::simulation::chunk::ChunkView::Dense(volume_view) => {
            for x in 0..size {
                for z in 0..size {
                    for y in 0..size {
                        data.push(volume_view.get_data(x, y, z));
                    }
                }
            }
        }
    }
    data
}

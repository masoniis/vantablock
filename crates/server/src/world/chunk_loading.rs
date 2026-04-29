use crate::network::systems::ClientConnection;
use crate::prelude::*;
use crate::world::chunk::datagen::gentask_components::NeedsGenerating;
use crate::world::chunk::manager::ServerChunkManager;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::network::ChunkData;
use shared::network::protocol::server::ServerMessage;
use shared::world::chunk::{
    ChunkBlocksComponent, ChunkCoord, ChunkLod, LOAD_DISTANCE, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
};
use std::collections::HashSet;

/// Tracks which chunks have been sent to a specific client.
#[derive(Component, Default)]
pub struct ClientChunkTracker {
    pub sent_chunks: HashSet<IVec3>,
}

/// Determines which chunks each player needs and starts loading them.
pub fn manage_player_chunk_loading_system(
    // input
    player_query: Query<(&Transform, Entity), With<ClientConnection>>,

    // output
    mut chunk_manager: ResMut<ServerChunkManager>,
    mut commands: Commands,
) {
    for (transform, player_ent) in player_query.iter() {
        trace!(target:"server_chunk_loading", "Processing player {:?} at {:?}", player_ent, transform.translation);
        let player_pos = transform.translation;
        let player_chunk_pos = ChunkCoord::world_to_chunk_pos(player_pos);

        let mut chunks_spurred_this_frame = 0;
        const MAX_CHUNKS_SPURRED_PER_FRAME: usize = 32;

        // determine desired chunks
        for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
            for z in -LOAD_DISTANCE..=LOAD_DISTANCE {
                for x in -LOAD_DISTANCE..=LOAD_DISTANCE {
                    if chunks_spurred_this_frame >= MAX_CHUNKS_SPURRED_PER_FRAME {
                        break;
                    }

                    let coord = IVec3::new(player_chunk_pos.x + x, y, player_chunk_pos.z + z);

                    if !chunk_manager.is_chunk_present_or_loading(coord) {
                        trace!(target:"server_chunk_loading","Server: Marking chunk needs-generation at {:?}", coord);
                        let ent = commands
                            .spawn((
                                NeedsGenerating { lod: ChunkLod(0) },
                                ChunkCoord { pos: coord },
                            ))
                            .id();
                        chunk_manager.mark_as_needs_generating(coord, ent);
                        chunks_spurred_this_frame += 1;
                    }
                }
                if chunks_spurred_this_frame >= MAX_CHUNKS_SPURRED_PER_FRAME {
                    break;
                }
            }
            if chunks_spurred_this_frame >= MAX_CHUNKS_SPURRED_PER_FRAME {
                break;
            }
        }
    }
}

/// Sends generated chunk data to clients that need it.
pub fn sync_chunk_data_to_clients_system(
    // Input
    mut client_query: Query<(&Transform, &ClientConnection, &mut ClientChunkTracker)>,
    chunk_query: Query<(&ChunkCoord, &ChunkBlocksComponent)>,
    chunk_manager: Res<ServerChunkManager>,
    mut sender_query: Query<&mut MessageSender<ServerMessage>>,
) {
    for (transform, connection, mut tracker) in client_query.iter_mut() {
        let Ok(mut sender) = sender_query.get_mut(connection.client_entity) else {
            warn!("No MessageSender for client {:?}", connection.client_entity);
            continue;
        };

        let player_pos = transform.translation;
        let player_chunk_pos = ChunkCoord::world_to_chunk_pos(player_pos);

        let mut chunks_sent_this_frame = 0;
        const MAX_CHUNKS_SENT_PER_FRAME: usize = 4;

        // find chunks within load distance that haven't been sent yet
        for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
            for z in -LOAD_DISTANCE..=LOAD_DISTANCE {
                for x in -LOAD_DISTANCE..=LOAD_DISTANCE {
                    if chunks_sent_this_frame >= MAX_CHUNKS_SENT_PER_FRAME {
                        break;
                    }

                    let coord = IVec3::new(player_chunk_pos.x + x, y, player_chunk_pos.z + z);

                    if tracker.sent_chunks.contains(&coord) {
                        continue;
                    }

                    // check if chunk is generated
                    if let Some(state) = chunk_manager.get_state(coord)
                        && state.is_generated()
                    {
                        if let Some(chunk_ent) = chunk_manager.get_entity(coord)
                            && let Ok((_coord, blocks)) = chunk_query.get(chunk_ent)
                        {
                            let data = extract_block_data(blocks);
                            let original_len = data.len();

                            // compress the data using zstd
                            let compressed_data = match zstd::encode_all(&data[..], 3) {
                                Ok(compressed) => compressed,
                                Err(e) => {
                                    error!("Failed to compress chunk data for {:?}: {}", coord, e);
                                    data
                                }
                            };

                            trace!(target:"server_chunk_loading", "Sending chunk {:?} to client {:?} (compressed: {} -> {} bytes)", coord, connection.client_entity, original_len, compressed_data.len());
                            sender.send::<ChunkData>(ServerMessage::ChunkData {
                                coord: ChunkCoord { pos: coord },
                                data: compressed_data,
                            });

                            tracker.sent_chunks.insert(coord);
                            chunks_sent_this_frame += 1;
                        } else {
                            trace!("Chunk {:?} is generated but has no entity or blocks", coord);
                        }
                    }
                }
                if chunks_sent_this_frame >= MAX_CHUNKS_SENT_PER_FRAME {
                    break;
                }
            }
            if chunks_sent_this_frame >= MAX_CHUNKS_SENT_PER_FRAME {
                break;
            }
        }
    }
}

fn extract_block_data(blocks: &ChunkBlocksComponent) -> Vec<u8> {
    let view = blocks.get_view();
    let size = blocks.size();
    let mut data = Vec::with_capacity(size * size * size);

    match view {
        shared::world::chunk::ChunkView::Uniform(block_id) => {
            data.resize(size * size * size, block_id);
        }
        shared::world::chunk::ChunkView::Dense(volume_view) => {
            // Optimally structured loop (X, Z, Y)
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

use crate::network::types::ClientConnection;
use crate::prelude::*;
use crate::world::chunk::chunk_map::ChunkMap;
use crate::world::chunk::components::{ActiveChunk, NeedsGenerating};
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::network::ChunkData;
use shared::network::protocol::ServerMessage;
use shared::player::components::LogicalPosition;
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
    player_query: Query<(&LogicalPosition, Entity), With<ClientConnection>>,

    // output
    mut chunk_manager: ResMut<ChunkMap>,
    mut commands: Commands,
) {
    for (position, player_ent) in player_query.iter() {
        trace!(target:"server_chunk_loading", "Processing player {:?} at {:?}", player_ent, position.0);
        let player_pos = position.0;
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

                    if !chunk_manager.is_chunk_present(coord) {
                        trace!(target:"server_chunk_loading","Server: Marking chunk needs-generation at {:?}", coord);
                        let ent = commands
                            .spawn((NeedsGenerating, ChunkLod(0), ChunkCoord { pos: coord }))
                            .id();
                        chunk_manager.register_chunk(coord, ent);
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
    // input
    mut client_query: Query<(&LogicalPosition, &ClientConnection, &mut ClientChunkTracker)>,
    chunk_query: Query<(&ChunkCoord, &ChunkBlocksComponent), With<ActiveChunk>>,
    chunk_manager: Res<ChunkMap>,
    mut sender_query: Query<&mut MessageSender<ServerMessage>>,
) {
    for (position, connection, mut tracker) in client_query.iter_mut() {
        let Ok(mut sender) = sender_query.get_mut(connection.client_entity) else {
            warn!("No MessageSender for client {:?}", connection.client_entity);
            continue;
        };

        let player_pos = position.0;
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

                    // Look up the Entity from ChunkMap::get_chunk(coord)
                    if let Some(chunk_ent) = chunk_manager.get_chunk(coord) {
                        // Query that Entity in the ECS to see if it has the ActiveChunk component
                        if let Ok((_coord, blocks)) = chunk_query.get(chunk_ent) {
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

// INFO: ---------------
//         tests
// ---------------------

#[cfg(test)]
mod tests {
    use crate::network::types::ClientConnection;
    use crate::prelude::*;
    use crate::world::{
        chunk::chunk_map::ChunkMap,
        chunk::components::NeedsGenerating,
        chunk_loading::{ClientChunkTracker, manage_player_chunk_loading_system},
    };
    use bevy::prelude::*;
    use shared::world::chunk::ChunkCoord;

    #[test]
    fn test_manage_player_chunk_loading() {
        let mut app = App::new();

        // add minimal resources
        app.insert_resource(ChunkMap::default());

        // add the system
        app.add_systems(Update, manage_player_chunk_loading_system);

        // spawn a player
        let client_entity = app.world_mut().spawn_empty().id();
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 32.0, 0.0),
            shared::player::components::LogicalPosition(Vec3::new(0.0, 32.0, 0.0)),
            ClientConnection { client_entity },
            ClientChunkTracker::default(),
        ));

        // run enough times to ensure the chunk is loaded (due to per-frame quota)
        for _ in 0..200 {
            app.update();
        }

        // check if chunks were requested
        let chunk_manager = app.world().resource::<ChunkMap>();
        assert!(
            !chunk_manager.chunks.is_empty(),
            "Chunk map should not be empty after player spawn"
        );

        // verify we have a chunk at (0, 1, 0) - player is at Y=32 which is chunk Y=1
        let player_chunk_coord = IVec3::new(0, 1, 0);
        assert!(chunk_manager.is_chunk_present(player_chunk_coord));

        // verify the entity has NeedsGenerating
        let entity = chunk_manager.get_chunk(player_chunk_coord).unwrap();
        assert!(app.world().get::<NeedsGenerating>(entity).is_some());
        assert!(app.world().get::<ChunkCoord>(entity).is_some());
    }
}

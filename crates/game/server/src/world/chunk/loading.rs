use crate::network::{receive::ecs_messages::InboundRequestChunkMessage, types::ClientConnection};
use crate::prelude::*;
use crate::world::chunk::chunk_map::ChunkMap;
use crate::world::chunk::components::{ActiveChunk, ModifiedChunk, NeedsGenerating};
use bevy::prelude::*;
use common::network::ChunkData;
use common::network::protocol::ServerMessage;
use common::player::components::LogicalPosition;
use common::world::chunk::{
    ChunkBlocksComponent, ChunkCoord, ChunkLod, LOAD_DISTANCE, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
};
use lightyear::prelude::MessageSender;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

/// How far (in chunks, horizontally) from the player a requested chunk may be. Slightly larger
/// than the load distance since the client's predicted position runs ahead of the server's.
const REQUEST_DISTANCE: i32 = LOAD_DISTANCE + 2;

/// How far (in chunks, horizontally) a chunk may be from every player before it is unloaded.
/// Beyond the request distance so pending requests stay satisfiable, and with an extra ring of
/// slack so walking along the boundary doesn't thrash chunks in and out.
const UNLOAD_DISTANCE: i32 = REQUEST_DISTANCE + 1;

/// Chunk column offsets within the load distance, nearest first.
static LOAD_COLUMN_OFFSETS: LazyLock<Vec<IVec2>> = LazyLock::new(|| {
    let mut offsets: Vec<IVec2> = (-LOAD_DISTANCE..=LOAD_DISTANCE)
        .flat_map(|z| (-LOAD_DISTANCE..=LOAD_DISTANCE).map(move |x| IVec2::new(x, z)))
        .collect();
    offsets.sort_by_key(|o| (o.x.abs().max(o.y.abs()), o.x.abs() + o.y.abs()));
    offsets
});

/// Horizontal (chebyshev) chunk distance, which matches the square load area.
fn horizontal_distance(a: IVec3, b: IVec3) -> i32 {
    (a.x - b.x).abs().max((a.z - b.z).abs())
}

fn is_within_request_range(player_chunk: IVec3, coord: IVec3) -> bool {
    (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).contains(&coord.y)
        && horizontal_distance(player_chunk, coord) <= REQUEST_DISTANCE
}

/// Tracks the chunks a specific client has requested but not yet been sent.
///
/// The client re-requests any chunk it unloaded and needs again, so the server never has to
/// guess what the client still holds.
#[derive(Component, Default)]
pub struct ClientChunkTracker {
    pub pending: HashSet<IVec3>,
}

/// Determines which chunks each player needs and starts generating them, nearest first.
pub fn manage_player_chunk_loading_system(
    // input
    player_query: Query<(&LogicalPosition, Entity), With<ClientConnection>>,

    // output
    mut chunk_manager: ResMut<ChunkMap>,
    mut commands: Commands,
) {
    const MAX_CHUNKS_SPURRED_PER_FRAME: usize = 32;

    for (position, player_ent) in player_query.iter() {
        trace!(target:"server_chunk_loading", "Processing player {:?} at {:?}", player_ent, position.0);
        let player_chunk_pos = ChunkCoord::world_to_chunk_pos(position.0);

        let mut chunks_spurred_this_frame = 0;

        'columns: for offset in LOAD_COLUMN_OFFSETS.iter() {
            for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
                if chunks_spurred_this_frame >= MAX_CHUNKS_SPURRED_PER_FRAME {
                    break 'columns;
                }

                let coord = IVec3::new(
                    player_chunk_pos.x + offset.x,
                    y,
                    player_chunk_pos.z + offset.y,
                );

                if !chunk_manager.is_chunk_present(coord) {
                    trace!(target:"server_chunk_loading","Server: Marking chunk needs-generation at {:?}", coord);
                    let ent = commands
                        .spawn((NeedsGenerating, ChunkLod(0), ChunkCoord { pos: coord }))
                        .id();
                    chunk_manager.register_chunk(coord, ent);
                    chunks_spurred_this_frame += 1;
                }
            }
        }
    }
}

/// Unloads chunks that are out of range of every player, so server memory stays bounded as
/// players explore. Chunks players have edited stay loaded until chunk persistence exists.
pub fn unload_distant_chunks_system(
    // input
    player_query: Query<&LogicalPosition, With<ClientConnection>>,
    modified_query: Query<(), With<ModifiedChunk>>,

    // output
    mut chunk_manager: ResMut<ChunkMap>,
    mut commands: Commands,
) {
    let player_chunks: Vec<IVec3> = player_query
        .iter()
        .map(|position| ChunkCoord::world_to_chunk_pos(position.0))
        .collect();

    chunk_manager.chunks.retain(|coord, entity| {
        let near_a_player = player_chunks
            .iter()
            .any(|player_chunk| horizontal_distance(*player_chunk, *coord) <= UNLOAD_DISTANCE);

        if near_a_player || modified_query.contains(*entity) {
            return true;
        }

        trace!(target:"server_chunk_loading", "Unloading chunk {:?}", coord);
        // despawning also drops any in-flight generation task's receiver
        commands.entity(*entity).despawn();
        false
    });
}

/// Queues the chunks each client requested so they can be sent once generated, and starts
/// generating any the server doesn't have.
///
/// The client decides which chunks it needs, from its own (predicted) camera position, so it
/// can ask for chunks just outside the ring the server generates around its view of the
/// player. Without generating those here, they would stay pending forever and leave holes.
pub fn queue_client_chunk_requests_system(
    // input
    mut requests: MessageReader<InboundRequestChunkMessage>,

    // output
    mut player_query: Query<(&LogicalPosition, &ClientConnection, &mut ClientChunkTracker)>,
    mut chunk_manager: ResMut<ChunkMap>,
    mut commands: Commands,
) {
    if requests.is_empty() {
        return;
    }

    // requests arrive keyed by the client's connection entity, not its player entity
    let mut requests_by_client: HashMap<Entity, Vec<IVec3>> = HashMap::new();
    for request in requests.read() {
        requests_by_client
            .entry(request.player)
            .or_default()
            .push(request.coord.pos);
    }

    for (position, connection, mut tracker) in player_query.iter_mut() {
        let Some(coords) = requests_by_client.remove(&connection.client_entity) else {
            continue;
        };

        let player_chunk_pos = ChunkCoord::world_to_chunk_pos(position.0);
        for coord in coords {
            if is_within_request_range(player_chunk_pos, coord) {
                tracker.pending.insert(coord);
                if !chunk_manager.is_chunk_present(coord) {
                    let entity = commands
                        .spawn((NeedsGenerating, ChunkLod(0), ChunkCoord { pos: coord }))
                        .id();
                    chunk_manager.register_chunk(coord, entity);
                }
            } else {
                debug!(target:"server_chunk_loading", "Ignoring out of range chunk request {:?} from {:?}", coord, connection.client_entity);
            }
        }
    }
}

/// Sends requested chunks to clients once they are generated, nearest first.
pub fn sync_chunk_data_to_clients_system(
    // input
    mut client_query: Query<(&LogicalPosition, &ClientConnection, &mut ClientChunkTracker)>,
    chunk_query: Query<&ChunkBlocksComponent, With<ActiveChunk>>,
    chunk_manager: Res<ChunkMap>,
    mut sender_query: Query<&mut MessageSender<ServerMessage>>,
) {
    const MAX_CHUNKS_SENT_PER_FRAME: usize = 32;

    for (position, connection, mut tracker) in client_query.iter_mut() {
        let Ok(mut sender) = sender_query.get_mut(connection.client_entity) else {
            warn!("No MessageSender for client {:?}", connection.client_entity);
            continue;
        };

        let player_chunk_pos = ChunkCoord::world_to_chunk_pos(position.0);

        // forget requests the player has since moved away from, the client unloads those too
        tracker
            .pending
            .retain(|coord| is_within_request_range(player_chunk_pos, *coord));

        // pick the nearest requested chunks that are ready to send
        let mut ready: Vec<(i32, IVec3, Entity)> = tracker
            .pending
            .iter()
            .filter_map(|&coord| {
                let entity = chunk_manager.get_chunk(coord)?;
                chunk_query
                    .contains(entity)
                    .then(|| (horizontal_distance(player_chunk_pos, coord), coord, entity))
            })
            .collect();
        ready.sort_unstable_by_key(|(distance, coord, _)| (*distance, coord.y));

        for (_, coord, entity) in ready.into_iter().take(MAX_CHUNKS_SENT_PER_FRAME) {
            let Ok(blocks) = chunk_query.get(entity) else {
                continue;
            };

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

            tracker.pending.remove(&coord);
        }
    }
}

fn extract_block_data(blocks: &ChunkBlocksComponent) -> Vec<u8> {
    let view = blocks.get_view();
    let size = blocks.size();
    let mut data = Vec::with_capacity(size * size * size);

    match view {
        common::world::chunk::ChunkView::Uniform(block_id) => {
            data.resize(size * size * size, block_id);
        }
        common::world::chunk::ChunkView::Dense(volume_view) => {
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
        chunk::loading::{ClientChunkTracker, manage_player_chunk_loading_system},
    };
    use bevy::prelude::*;
    use common::world::chunk::ChunkCoord;

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
            common::player::components::LogicalPosition(Vec3::new(0.0, 32.0, 0.0)),
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

use crate::world::chunk::ChunkMap;
use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::{
    network::protocol::{BlockUpdates, ClientMessage, ServerMessage},
    world::block::block_registry::AIR_BLOCK_ID,
    world::chunk::CHUNK_SIDE_LENGTH,
    world::chunk::components::{ChunkBlocksComponent, ChunkCoord},
};

/// Handles requests from clients to modify voxels (break or place).
///
/// This system updates the authoritative server state and broadcasts the change
/// to all other connected clients as a `ServerMessage::VoxelUpdate`.
pub fn handle_client_voxel_requests(
    mut query: Query<(&RemoteId, &mut MessageReceiver<ClientMessage>)>,
    chunk_map: Res<ChunkMap>,
    mut blocks_query: Query<&mut ChunkBlocksComponent>,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Server>>,
) {
    let Some(server) = server else { return };

    for (remote_id, mut receiver) in query.iter_mut() {
        for message in receiver.receive() {
            let (world_pos, block_id) = match message {
                ClientMessage::BreakBlock { position } => (position, AIR_BLOCK_ID),
                ClientMessage::PlaceBlock { position, block_id } => (position, block_id),
                _ => continue, // Ignore other messages in this system
            };

            let chunk_pos = ChunkCoord::world_to_chunk_pos(world_pos.as_vec3());

            if let Some(entity) = chunk_map.get_chunk(chunk_pos)
                && let Ok(mut chunk_blocks) = blocks_query.get_mut(entity)
            {
                let local_pos = world_pos - (chunk_pos * CHUNK_SIDE_LENGTH as i32);

                let mut writer = chunk_blocks.get_writer();
                writer.set_data(
                    local_pos.x as usize,
                    local_pos.y as usize,
                    local_pos.z as usize,
                    block_id,
                );

                // broadcast state change to all other clients.
                let update_message = ServerMessage::VoxelUpdate {
                    position: world_pos,
                    block_id,
                };

                let _ = sender.send::<_, BlockUpdates>(
                    &update_message,
                    *server,
                    &NetworkTarget::AllExcept(vec![**remote_id]),
                );
            }
        }
    }
}

use crate::{
    network::receive::ecs_messages::{InboundBreakBlockMessage, InboundPlaceBlockMessage},
    world::chunk::ChunkMap,
};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::{
    network::protocol::{BlockUpdates, ServerMessage},
    world::block::block_registry::AIR_BLOCK_ID,
    world::chunk::CHUNK_SIDE_LENGTH,
    world::chunk::components::{ChunkBlocksComponent, ChunkCoord},
};

/// Handles requests from clients to modify blocks (break or place).
///
/// This system updates the authoritative server state and broadcasts the change
/// to all other connected clients as a `ServerMessage::BlockUpdate`.
pub fn handle_client_block_requests(
    mut ev_break: MessageReader<InboundBreakBlockMessage>,
    mut ev_place: MessageReader<InboundPlaceBlockMessage>,
    chunk_map: Res<ChunkMap>,
    mut blocks_query: Query<&mut ChunkBlocksComponent>,
    mut sender: ServerMultiMessageSender,
    server: Option<Single<&Server>>,
    player_query: Query<&RemoteId>,
) {
    let Some(server) = server else { return };

    let mut process_request = |player_entity: Entity, world_pos: IVec3, block_id: u8| {
        let Ok(remote_id) = player_query.get(player_entity) else {
            return;
        };

        let chunk_pos = ChunkCoord::world_to_chunk_pos(world_pos.as_vec3());

        if let Some(entity) = chunk_map.get_chunk(chunk_pos)
            && let Ok(mut chunk_blocks) = blocks_query.get_mut(entity)
        {
            let local_pos = world_pos - (chunk_pos * CHUNK_SIDE_LENGTH as i32);

            info!("Updating server block state at chunk {:?} local {:?}", chunk_pos, local_pos);

            let mut writer = chunk_blocks.get_writer();
            writer.set_data(
                local_pos.x as usize,
                local_pos.y as usize,
                local_pos.z as usize,
                block_id,
            );

            // broadcast state change to all other clients.
            let update_message = ServerMessage::BlockUpdate {
                position: world_pos,
                block_id,
            };

            let _ = sender.send::<_, BlockUpdates>(
                &update_message,
                *server,
                &NetworkTarget::AllExcept(vec![**remote_id]),
            );
        } else {
            warn!("Could not find chunk or blocks for position {:?}", world_pos);
        }
    };

    for ev in ev_break.read() {
        info!("Server received BreakBlock at {:?} from player {:?}", ev.position, ev.player);
        process_request(ev.player, ev.position, AIR_BLOCK_ID);
    }

    for ev in ev_place.read() {
        info!("Server received PlaceBlock at {:?} (id: {}) from player {:?}", ev.position, ev.block_id, ev.player);
        process_request(ev.player, ev.position, ev.block_id);
    }
}

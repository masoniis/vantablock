use crate::network::messages::ReceivedChunkDataEvent;
use crate::render::chunk::manager::{ClientChunkManager, ClientChunkState};
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use shared::world::chunk::{ChunkBlocksComponent, ChunkCoord, ChunkLod};

pub fn apply_received_chunk_data_system(
    mut ev_chunk: MessageReader<ReceivedChunkDataEvent>,
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    for event in ev_chunk.read() {
        let coord = event.coord.pos;
        let data = &event.data;

        let state = chunk_manager.get_state(coord);
        match state {
            Some(ClientChunkState::AwaitingData) => {
                trace!(target: "client_network", "Applying received chunk data for {:?}", coord);

                let blocks = ChunkBlocksComponent::from_vec(ChunkLod(0), data.clone());

                // spawn the entity now that we have data
                let entity = commands.spawn((ChunkCoord { pos: coord }, blocks)).id();

                chunk_manager.mark_as_data_ready(coord, entity);
            }
            Some(_) => {
                // already have data or meshing, maybe an update?
                // for now we don't handle updates via this system
                debug!(
                    "Received chunk data for chunk at {:?} but it's already in state {:?}",
                    coord, state
                );
            }
            None => {
                warn!("Received chunk data for untracked chunk at {:?}", coord);
            }
        }
    }
}

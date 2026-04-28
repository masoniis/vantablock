use crate::network::messages::ReceivedChunkDataEvent;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use shared::simulation::chunk::{ChunkBlocksComponent, ChunkLod, ChunkStateManager};

pub fn apply_received_chunk_data_system(
    mut ev_chunk: MessageReader<ReceivedChunkDataEvent>,
    chunk_manager: Res<ChunkStateManager>,
    mut commands: Commands,
) {
    for event in ev_chunk.read() {
        let coord = event.coord.pos;
        let data = &event.data;

        if let Some(entity) = chunk_manager.get_entity(coord) {
            trace!(target: "client_network", "Applying received chunk data for {:?}", coord);
            
            // create chunk component from raw data
            // assuming data is a dense array of u16 block IDs (or u8 if that's what we sent)
            // in ServerMessage::ChunkData it was Vec<u8>
            // extract_block_data in server sent Vec<u8> where each u8 is a block_id
            
            let blocks = ChunkBlocksComponent::from_vec(ChunkLod(0), data.clone());
            
            commands.entity(entity).insert(blocks);
            
            // note: promote_newly_generated_chunks_system in render/chunk/mod.rs 
            // will catch the Added<ChunkBlocksComponent> and move it to WantsMeshing
        } else {
            warn!("Received chunk data for untracked chunk at {:?}", coord);
        }
    }
}

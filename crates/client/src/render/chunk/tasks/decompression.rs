use crate::{
    network::InboundCompressedChunkMessage,
    render::{
        chunk::manager::{ClientChunkManager, ClientChunkState},
        chunk::tasks::components::DecompressionTask,
    },
};
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use futures_lite::future;
use shared::world::chunk::{ChunkBlocksComponent, ChunkCoord, ChunkLod};

#[derive(Message)]
pub struct DecompressedChunkMessage {
    pub coord: ChunkCoord,
    pub data: Vec<u8>,
}

pub fn decompress_chunk_data_system(
    mut commands: Commands,
    mut ev_compressed: MessageReader<InboundCompressedChunkMessage>,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    for event in ev_compressed.read() {
        let coord = event.coord.clone();
        let data = event.data.clone();

        let task = thread_pool.spawn(async move {
            match zstd::decode_all(&data[..]) {
                Ok(decompressed) => (coord, decompressed),
                Err(e) => {
                    error!("Failed to decompress chunk data for {:?}: {}", coord, e);
                    (coord, Vec::new())
                }
            }
        });

        commands.spawn(DecompressionTask(task));
    }
}

pub fn poll_decompression_tasks_system(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut DecompressionTask)>,
    mut ev_decompressed: MessageWriter<DecompressedChunkMessage>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some((coord, data)) = future::block_on(future::poll_once(&mut task.0)) {
            if !data.is_empty() {
                ev_decompressed.write(DecompressedChunkMessage { coord, data });
            }
            commands.entity(entity).despawn();
        }
    }
}

pub fn apply_decompressed_chunk_data_system(
    mut ev_chunk: MessageReader<DecompressedChunkMessage>,
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    for event in ev_chunk.read() {
        let coord = event.coord.pos;
        let data = &event.data;

        let state = chunk_manager.get_state(coord);
        match state {
            Some(ClientChunkState::AwaitingData) => {
                trace!(target: "client_network", "Applying decompressed chunk data for {:?}", coord);

                let blocks = ChunkBlocksComponent::from_vec(ChunkLod(0), data.clone());

                // spawn the entity now that we have data
                let entity = commands.spawn((ChunkCoord { pos: coord }, blocks)).id();

                chunk_manager.mark_as_data_ready(coord, entity);
            }
            Some(_) => {
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

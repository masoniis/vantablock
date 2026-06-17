use super::ChunkGenerationTaskComponent;
use crate::{
    prelude::*,
    world::chunk::chunk_map::ChunkMap,
    world::chunk::components::{ActiveChunk, EmptyChunk, Generating},
};
use bevy::ecs::prelude::*;
use crossbeam::channel::TryRecvError;
use shared::world::chunk::ChunkCoord;

/// Polls chunk generation tasks, adds generated components, and marks chunks as `Active`.
#[instrument(skip_all)]
pub fn poll_chunk_generation_tasks(
    // input
    mut tasks_query: Query<
        (Entity, &mut ChunkGenerationTaskComponent, &ChunkCoord),
        With<Generating>,
    >,

    // output
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkMap>,
) {
    // poll all generation tasks
    for (entity, generation_task_component, coord) in tasks_query.iter_mut() {
        match generation_task_component.receiver.try_recv() {
            Ok(gen_bundle) => {
                if let Some(chunk_blocks) = gen_bundle.chunk_blocks {
                    trace!(
                        target: "chunk_loading",
                        "Chunk generation finished for {}. Marking as Active.",
                        coord
                    );
                    commands
                        .entity(entity)
                        .insert((chunk_blocks, gen_bundle.biome_map, ActiveChunk))
                        .remove::<ChunkGenerationTaskComponent>()
                        .remove::<Generating>();
                } else {
                    trace!(
                        target: "chunk_loading",
                        "Chunk generation finished for {} but chunk is empty. Marking as Active (Empty).",
                        coord
                    );
                    commands
                        .entity(entity)
                        .insert(EmptyChunk)
                        .remove::<ChunkGenerationTaskComponent>()
                        .remove::<Generating>();
                }
            }
            Err(TryRecvError::Empty) => {
                // task still running
            }
            Err(TryRecvError::Disconnected) => {
                warn!(
                    target: "chunk_loading",
                    "Chunk generation task for {} failed (channel disconnected). Despawning entity.",
                    coord
                );
                commands.entity(entity).despawn();
                chunk_manager.unregister_chunk(coord.pos);
            }
        }
    }
}

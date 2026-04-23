use crate::prelude::*;
use crate::simulation::chunk::ChunkGenerationTaskComponent;
use bevy::ecs::prelude::*;
use crossbeam::channel::TryRecvError;
use shared::simulation::chunk::{ChunkCoord, ChunkState, ChunkStateManager};

/// Polls chunk generation tasks, adds generated components, and marks chunks as `DataReady`.
#[instrument(skip_all)]
pub fn poll_chunk_generation_tasks(
    // Input
    mut tasks_query: Query<(Entity, &mut ChunkGenerationTaskComponent, &ChunkCoord)>,

    // Output
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkStateManager>,
) {
    // poll all generation tasks
    for (entity, generation_task_component, coord) in tasks_query.iter_mut() {
        match generation_task_component.receiver.try_recv() {
            Ok(gen_bundle) => {
                let current_state = chunk_manager.get_state(coord.pos);
                match current_state {
                    Some(ChunkState::Generating { entity: gen_entity }) if gen_entity == entity => {
                        if let Some(chunk_blocks) = gen_bundle.chunk_blocks {
                            trace!(
                                target: "chunk_loading",
                                "Chunk generation finished for {}. Marking as DataReady.",
                                coord
                            );
                            commands
                                .entity(entity)
                                .insert((chunk_blocks, gen_bundle.biome_map))
                                .remove::<ChunkGenerationTaskComponent>();
                            chunk_manager.mark_as_data_ready(coord.pos, entity);
                        } else {
                            trace!(
                                target: "chunk_loading",
                                "Chunk generation finished for {} but chunk is empty. Marking as Loaded(None).",
                                coord
                            );
                            commands.entity(entity).despawn();
                            chunk_manager.mark_as_loaded_but_empty(coord.pos);
                        }

                        // Neighbors that may be waiting for this chunk will be handled by client-side meshing logic.
                    }
                    Some(_) => {
                        error!(
                            "Chunk generation task for {} completed, but manager state changed unexpectedly to {:?}.",
                            coord, current_state
                        );
                    }
                    None => {
                        debug!(target: "chunk_loading", "Generation completed for unloaded chunk {}, cleaning up", coord);
                        commands
                            .entity(entity)
                            .remove::<ChunkGenerationTaskComponent>();
                    }
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
                chunk_manager.mark_as_unloaded(coord.pos);
            }
        }
    }
}

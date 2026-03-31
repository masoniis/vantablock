use crate::prelude::*;
use crate::simulation_world::chunk::{
    CheckForMeshing, ChunkGenerationTaskComponent, ChunkState, WantsMeshing, RENDER_DISTANCE,
    WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
};
use crate::simulation_world::chunk::{ChunkCoord, ChunkStateManager};
use crate::simulation_world::player::ActiveCamera;
use bevy::ecs::prelude::*;
use crossbeam::channel::TryRecvError;

/// Assesses whether a chunk coordinate is within the meshing radius of the camera.
pub fn chunk_is_in_mesh_radius(camera_chunk_pos: IVec3, chunk_coord: IVec3) -> bool {
    let dx = chunk_coord.x - camera_chunk_pos.x;
    let dy = chunk_coord.y;
    let dz = chunk_coord.z - camera_chunk_pos.z;

    dx.abs() <= RENDER_DISTANCE
        && (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).contains(&dy)
        && dz.abs() <= RENDER_DISTANCE
}

/// Polls chunk generation tasks, adds generated components, and marks chunks as
/// `NeedsMeshing` (if in range) or `DataReady` (if out of range).
#[instrument(skip_all)]
pub fn poll_chunk_generation_tasks(
    // Input
    mut tasks_query: Query<(Entity, &mut ChunkGenerationTaskComponent, &ChunkCoord)>,
    active_camera: Res<ActiveCamera>, // to gauge if chunk is in meshing range
    camera_query: Query<&ChunkCoord>,

    // Output
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkStateManager>,
) {
    let camera_chunk_pos = camera_query.get(active_camera.0).map(|c| c.pos);

    // poll all generation tasks
    for (entity, generation_task_component, coord) in tasks_query.iter_mut() {
        match generation_task_component.receiver.try_recv() {
            Ok(gen_bundle) => {
                let current_state = chunk_manager.get_state(coord.pos);
                match current_state {
                    Some(ChunkState::Generating { entity: gen_entity }) if gen_entity == entity => {
                        if let Some(chunk_blocks) = gen_bundle.chunk_blocks {
                            let mut is_in_mesh_radius = false;
                            if let Ok(cam_pos) = camera_chunk_pos {
                                is_in_mesh_radius = chunk_is_in_mesh_radius(cam_pos, coord.pos);
                            }

                            if is_in_mesh_radius {
                                trace!(
                                    target: "chunk_loading",
                                    "Chunk generation finished for {}. In range. Marking as NeedsMeshing.",
                                    coord
                                );
                                commands
                                    .entity(entity)
                                    .insert((
                                        chunk_blocks,
                                        gen_bundle.biome_map,
                                        WantsMeshing,
                                        CheckForMeshing,
                                    ))
                                    .remove::<ChunkGenerationTaskComponent>();
                                chunk_manager.mark_as_needs_meshing(coord.pos, entity);
                            } else {
                                trace!(
                                    target: "chunk_loading",
                                    "Chunk generation finished for {}. Out of range. Marking as DataReady.",
                                    coord
                                );
                                commands
                                    .entity(entity)
                                    .insert((chunk_blocks, gen_bundle.biome_map))
                                    .remove::<ChunkGenerationTaskComponent>();
                                chunk_manager.mark_as_data_ready(coord.pos, entity);
                            }
                        } else {
                            trace!(
                                target: "chunk_loading",
                                "Chunk generation finished for {} but chunk is empty. Marking as Loaded(None).",
                                coord
                            );
                            commands.entity(entity).despawn();
                            chunk_manager.mark_as_loaded_but_empty(coord.pos);
                        }

                        // ping any neighbors that may have been waiting on this chunk
                        for neighbor in chunk_manager.iter_neighbors(coord.pos) {
                            if let ChunkState::WantsMeshing { .. } = neighbor.state {
                                commands.entity(neighbor.entity).insert(CheckForMeshing);
                            }
                        }
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

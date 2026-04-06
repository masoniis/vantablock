use crate::prelude::*;
use crate::simulation::asset_management::VoxelChunkMeshAsset;
use crate::simulation::chunk::{
    CHUNK_SIDE_LENGTH, CheckForMeshing, ChunkCoord, ChunkMeshingTaskComponent, ChunkState,
    ChunkStateManager, OpaqueMeshComponent, TransformComponent, TransparentMeshComponent,
    WantsMeshing,
};
use bevy::asset::Assets;
use bevy::ecs::prelude::*;
use crossbeam::channel::TryRecvError;

/// Polls chunk meshing tasks and adds the MeshComponent when ready.
#[instrument(skip_all)]
pub fn poll_chunk_meshing_tasks(
    // Input
    mut tasks_query: Query<(Entity, &mut ChunkMeshingTaskComponent, &ChunkCoord)>,
    existing_meshes: Query<(
        Option<&OpaqueMeshComponent>,
        Option<&TransparentMeshComponent>,
    )>,

    // Output
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkStateManager>,
    mut mesh_assets: ResMut<Assets<VoxelChunkMeshAsset>>,
) {
    // poll all mesh task
    for (entity, meshing_task_component, coord) in tasks_query.iter_mut() {
        match meshing_task_component.receiver.try_recv() {
            Ok((opaque_asset_option, transparent_asset_option)) => {
                let current_state = chunk_manager.get_state(coord.pos);

                match current_state {
                    Some(
                        ChunkState::Meshing { entity: gen_entity }
                        | ChunkState::WantsMeshing { entity: gen_entity },
                    ) if gen_entity == entity => {
                        trace!(target : "chunk_loading","Chunk meshing finished for {:?}", coord);

                        // remove old if it exists
                        let (exists_opaque, exists_transparent) =
                            existing_meshes.get(entity).unwrap_or((None, None));

                        if exists_opaque.is_some() {
                            commands.entity(entity).remove::<OpaqueMeshComponent>();
                        }
                        if exists_transparent.is_some() {
                            commands.entity(entity).remove::<TransparentMeshComponent>();
                        }

                        // add new comps to asset storage
                        let opaque_component = if let Some(asset) = opaque_asset_option {
                            let handle = mesh_assets.add(asset);
                            Some(OpaqueMeshComponent::new(handle))
                        } else {
                            None
                        };

                        let transparent_component = if let Some(asset) = transparent_asset_option {
                            let handle = mesh_assets.add(asset);
                            Some(TransparentMeshComponent::new(handle))
                        } else {
                            None
                        };

                        // apply to entity
                        match (opaque_component, transparent_component) {
                            (Some(opaque_mesh), Some(transparent_mesh)) => {
                                commands
                                    .entity(entity)
                                    .insert((opaque_mesh, transparent_mesh));
                            }
                            (Some(opaque_mesh), None) => {
                                commands.entity(entity).insert(opaque_mesh);
                            }
                            (None, Some(transparent_mesh)) => {
                                commands.entity(entity).insert(transparent_mesh);
                            }
                            (None, None) => {
                                commands
                                    .entity(entity)
                                    .remove::<ChunkMeshingTaskComponent>();
                                chunk_manager.mark_as_loaded(coord.pos, entity);
                                continue; // continue to avoid adding transform component
                            }
                        }

                        // insert to world
                        commands
                            .entity(entity)
                            .insert(TransformComponent {
                                position: Vec3::new(
                                    (coord.x * CHUNK_SIDE_LENGTH as i32) as f32,
                                    (coord.y * CHUNK_SIDE_LENGTH as i32) as f32,
                                    (coord.z * CHUNK_SIDE_LENGTH as i32) as f32,
                                ),
                                rotation: Quat::IDENTITY,
                                scale: Vec3::ONE,
                            })
                            .remove::<ChunkMeshingTaskComponent>();

                        chunk_manager.mark_as_loaded(coord.pos, entity);
                    }
                    Some(_) => {
                        error!(
                            "Chunk meshing task for {} completed but manager state entity does not match ({:?} != {:?}).",
                            coord,
                            current_state.unwrap().entity(),
                            entity
                        );
                    }
                    _ => {
                        debug!(
                            target : "chunk_loading",
                            "Mesh generation completed for unloaded chunk coord {}. Discarding assets and cleaning up entity {}.",
                            coord, entity
                        );

                        commands
                            .entity(entity)
                            .remove::<ChunkMeshingTaskComponent>();
                        continue;
                    }
                }
            }
            Err(TryRecvError::Empty) => {
                // task still running
            }
            Err(TryRecvError::Disconnected) => {
                warn!(
                    target: "chunk_loading",
                    "Chunk meshing task for {} failed (channel disconnected). Despawning entity.",
                    coord
                );
                // try to send it to be remeshed
                chunk_manager.mark_as_needs_meshing(coord.pos, entity);
                commands
                    .entity(entity)
                    .remove::<ChunkMeshingTaskComponent>()
                    .insert(CheckForMeshing)
                    .insert(WantsMeshing);
            }
        }
    }
}

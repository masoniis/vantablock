use crate::prelude::*;
use crate::render::chunk::{
    BlockMeshAsset, ClientChunkManager, ClientChunkState, OpaqueMeshComponent,
    TransparentMeshComponent,
    tasks::{CheckForMeshing, WantsMeshing, components::ChunkMeshingTaskComponent},
};
use bevy::{asset::Assets, ecs::prelude::*, prelude::*};
use common::world::chunk::{CHUNK_SIDE_LENGTH, ChunkCoord};
use crossbeam::channel::TryRecvError;

/// Polls chunk meshing tasks and adds the MeshComponent when ready.
#[instrument(skip_all)]
pub fn poll_chunk_meshing_tasks(
    // input
    mut tasks_query: Query<(Entity, &mut ChunkMeshingTaskComponent, &ChunkCoord)>,
    existing_meshes: Query<(
        Option<&OpaqueMeshComponent>,
        Option<&TransparentMeshComponent>,
    )>,

    // output
    mut commands: Commands,
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut mesh_assets: ResMut<Assets<BlockMeshAsset>>,
) {
    // poll all mesh task
    for (entity, meshing_task_component, coord) in tasks_query.iter_mut() {
        match meshing_task_component.receiver.try_recv() {
            Ok((opaque_asset_option, transparent_asset_option)) => {
                let current_state = chunk_manager.get_state(coord.pos);

                match current_state {
                    Some(
                        state @ (ClientChunkState::Meshing { entity: gen_entity }
                        | ClientChunkState::NeedsMeshing { entity: gen_entity }),
                    ) if gen_entity == entity => {
                        // an edit that landed while this task ran already queued a remesh. This
                        // (now stale) mesh is still shown in the meantime, but the state must
                        // stay `NeedsMeshing` or the queued remesh is treated as cancelled.
                        let remesh_pending = matches!(state, ClientChunkState::NeedsMeshing { .. });

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
                                // chunk has no visual meshes, still need to mark it as mesh complete
                                commands
                                    .entity(entity)
                                    .remove::<ChunkMeshingTaskComponent>();
                                if !remesh_pending {
                                    chunk_manager.mark_as_mesh_complete(coord.pos, entity);
                                }
                                continue; // continue to prevent transform from being added
                            }
                        }

                        // insert to world
                        commands
                            .entity(entity)
                            .insert((
                                Transform::from_translation(Vec3::new(
                                    (coord.x * CHUNK_SIDE_LENGTH as i32) as f32,
                                    (coord.y * CHUNK_SIDE_LENGTH as i32) as f32,
                                    (coord.z * CHUNK_SIDE_LENGTH as i32) as f32,
                                )),
                                Visibility::default(),
                            ))
                            .remove::<ChunkMeshingTaskComponent>();

                        if !remesh_pending {
                            chunk_manager.mark_as_mesh_complete(coord.pos, entity);
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::chunk::meshing::{OpaqueMeshData, TransparentMeshData};
    use crossbeam::channel::unbounded;

    /// Runs `poll_chunk_meshing_tasks` once for a chunk whose in-flight task returns `result`,
    /// with the chunk manager already in `state`, and returns the state afterwards.
    fn poll_with_state(
        state: fn(Entity) -> ClientChunkState,
        result: (Option<OpaqueMeshData>, Option<TransparentMeshData>),
    ) -> ClientChunkState {
        let coord = IVec3::new(2, 1, 3);
        let mut app = App::new();
        app.init_resource::<ClientChunkManager>();
        app.init_resource::<Assets<BlockMeshAsset>>();
        app.add_systems(Update, poll_chunk_meshing_tasks);

        let (sender, receiver) = unbounded();
        sender.send(result).unwrap();
        let entity = app
            .world_mut()
            .spawn((
                ChunkCoord { pos: coord },
                ChunkMeshingTaskComponent { receiver },
            ))
            .id();
        app.world_mut()
            .resource_mut::<ClientChunkManager>()
            .chunk_states
            .insert(coord, state(entity));

        app.update();

        app.world()
            .resource::<ClientChunkManager>()
            .get_state(coord)
            .unwrap()
    }

    fn some_mesh() -> OpaqueMeshData {
        BlockMeshAsset {
            name: "test".into(),
            faces: vec![crate::render::chunk::meshing::PackedFace(0)].into(),
        }
    }

    #[test]
    fn finished_mesh_completes_a_meshing_chunk() {
        for result in [(None, None), (Some(some_mesh()), None)] {
            let state = poll_with_state(|entity| ClientChunkState::Meshing { entity }, result);
            assert!(matches!(state, ClientChunkState::MeshComplete { .. }));
        }
    }

    /// Regression: an edit during meshing marks the chunk `NeedsMeshing`. The finished (stale)
    /// mesh used to overwrite that with `MeshComplete`, so the edit never got remeshed.
    #[test]
    fn edit_during_meshing_keeps_the_remesh_queued() {
        for result in [(None, None), (Some(some_mesh()), None)] {
            let state = poll_with_state(|entity| ClientChunkState::NeedsMeshing { entity }, result);
            assert!(matches!(state, ClientChunkState::NeedsMeshing { .. }));
        }
    }
}

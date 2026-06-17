use crate::prelude::*;
use crate::render::{
    block::BlockRenderDataRegistry,
    chunk::{
        BlockMeshAsset, ClientChunkManager, ClientChunkState, OpaqueMeshComponent,
        TransparentMeshComponent,
        meshing::build_chunk_mesh,
        tasks::{CheckForMeshing, WantsMeshing, components::ChunkMeshingTaskComponent},
    },
};
use bevy::{asset::Assets, ecs::prelude::*, prelude::*, tasks::AsyncComputeTaskPool};
use crossbeam::channel::{TryRecvError, unbounded};
use shared::world::{
    block::BlockRegistry,
    chunk::{
        CHUNK_SIDE_LENGTH, ChunkBlocksComponent, ChunkCoord, NEIGHBOR_OFFSETS, RENDER_DISTANCE,
        WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
        common::{
            chunk_scaling::{downsample_chunk, upsample_chunk},
            padded_chunk_view::{ChunkDataOption, NeighborLODs, PaddedChunk},
            thread_buffer_pool::{acquire_buffer, release_buffer},
        },
    },
};
use std::collections::HashSet;

/// Determines if a coord is in bounds
pub fn is_in_bounds(coord: IVec3) -> bool {
    let pos_y = coord.y;
    (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).contains(&pos_y)
}

/// Queries for chunks needing meshing and starts a limited number of tasks per frame.
#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
pub fn start_pending_meshing_tasks_system(
    mut pending_chunks_query: Query<
        (Entity, &ChunkBlocksComponent, &ChunkCoord),
        (
            With<WantsMeshing>,
            With<CheckForMeshing>,
            Without<ChunkMeshingTaskComponent>,
        ),
    >,
    all_generated_chunks: Query<&ChunkBlocksComponent>, // for finding neighbors

    // Resources needed to start meshing
    mut commands: Commands,
    mut chunk_manager: ResMut<ClientChunkManager>,
    block_registry: Res<BlockRegistry>,
    render_registry: Res<BlockRenderDataRegistry>,
) {
    'chunk_loop: for (entity, chunk_comp, chunk_coord) in pending_chunks_query.iter_mut() {
        // check for cancellation
        match chunk_manager.get_state(chunk_coord.pos) {
            Some(ClientChunkState::NeedsMeshing {
                entity: state_entity,
            }) if state_entity == entity => {
                // state is correct, proceed to start meshing
            }
            _ => {
                debug!(
                    target : "chunk_loading",
                    "Chunk {} marked NeedsMeshing but manager state is not NeedsMeshing({:?}). Assuming cancelled.",
                    chunk_coord.pos, entity
                );
                continue;
            }
        }

        // INFO: ----------------------------------------------
        //         ensure neighbors have been generated
        // ----------------------------------------------------

        enum NeighborStatus {
            Ready(ChunkDataOption),
            WaitingForGeneration,
            WaitingForDeferredCommands,
        }

        let get_neighbor = |offset: IVec3| -> NeighborStatus {
            let neighbor_coord = chunk_coord.pos + offset;

            if !is_in_bounds(neighbor_coord) {
                return NeighborStatus::Ready(ChunkDataOption::OutOfBounds);
            }

            match chunk_manager.get_state(neighbor_coord) {
                Some(state) if state.is_generated() => {
                    let entity = state.entity().expect("Expected entity for generated chunk");
                    match all_generated_chunks.get(entity) {
                        Ok(blocks) => {
                            NeighborStatus::Ready(ChunkDataOption::Generated(blocks.clone()))
                        }
                        Err(_) => NeighborStatus::WaitingForDeferredCommands,
                    }
                }
                _ => NeighborStatus::WaitingForGeneration,
            }
        };

        // INFO: ------------------------------------------------------
        //         Create neighbor data (including LOD scaling)
        // ------------------------------------------------------------

        let mut chunks: [[[ChunkDataOption; 3]; 3]; 3] = Default::default();
        chunks[1][1][1] = ChunkDataOption::Generated(chunk_comp.clone()); // center chunk
        let center_lod = chunk_comp.lod();

        let mut original_neighbor_lods = NeighborLODs::default();

        for chunk_offset in NEIGHBOR_OFFSETS {
            let neighbor_data = match get_neighbor(chunk_offset) {
                NeighborStatus::Ready(data) => data,
                NeighborStatus::WaitingForGeneration => {
                    // abort for now, we will be pinged once it finishes generating
                    commands.entity(entity).remove::<CheckForMeshing>();
                    continue 'chunk_loop;
                }
                NeighborStatus::WaitingForDeferredCommands => {
                    // neighbors ARE ready, but their components aren't visible yet.
                    // DO NOT remove CheckForMeshing so we try again next frame!
                    continue 'chunk_loop;
                }
            };

            let (processed_data, original_lod) = match neighbor_data {
                ChunkDataOption::Generated(neighbor_blocks) => {
                    let neighbor_lod = neighbor_blocks.lod();

                    let processed_blocks = if neighbor_lod > center_lod {
                        ChunkDataOption::Generated(upsample_chunk(&neighbor_blocks, center_lod))
                    } else if neighbor_lod < center_lod {
                        ChunkDataOption::Generated(downsample_chunk(&neighbor_blocks, center_lod))
                    } else {
                        ChunkDataOption::Generated(neighbor_blocks.clone())
                    };

                    (processed_blocks, Some(neighbor_lod))
                }
                ChunkDataOption::OutOfBounds => (ChunkDataOption::OutOfBounds, None),
                ChunkDataOption::Empty => (ChunkDataOption::Empty, None),
            };

            // map offset (e.g., -1, 0, 1) to array index (e.g., 0, 1, 2)
            let idx_x = (chunk_offset.x + 1) as usize;
            let idx_y = (chunk_offset.y + 1) as usize;
            let idx_z = (chunk_offset.z + 1) as usize;

            chunks[idx_x][idx_y][idx_z] = processed_data;
            original_neighbor_lods[idx_x][idx_y][idx_z] = original_lod;
        }

        // INFO: -----------------------------
        //         Spawn the mesh task
        // -----------------------------------

        let block_registry_clone = block_registry.clone();
        let render_registry_clone = render_registry.clone();
        let coord_clone = chunk_coord.clone();

        trace!(target: "chunk_loading", "Starting meshing task for {}.", chunk_coord.pos);

        let (sender, receiver) = unbounded();

        AsyncComputeTaskPool::get()
            .spawn(async move {
                let buffer = acquire_buffer();

                let padded_view =
                    PaddedChunk::new(&chunks, center_lod, original_neighbor_lods, buffer);

                let texture_lut = render_registry_clone.get_texture_lut();

                let (opaque_mesh_option, transparent_mesh_option) = build_chunk_mesh(
                    &coord_clone.to_string(),
                    &padded_view,
                    &block_registry_clone,
                    &render_registry_clone,
                    texture_lut,
                );

                let used_buffer = padded_view.take_buffer();
                release_buffer(used_buffer);

                let _ = sender.send((opaque_mesh_option, transparent_mesh_option));
            })
            .detach();

        // update entity and manager
        commands
            .entity(entity)
            .insert(ChunkMeshingTaskComponent { receiver })
            .remove::<CheckForMeshing>()
            .remove::<WantsMeshing>();

        chunk_manager.mark_as_meshing(chunk_coord.pos, entity);
    }
}

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
                        ClientChunkState::Meshing { entity: gen_entity }
                        | ClientChunkState::NeedsMeshing { entity: gen_entity },
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
                                // chunk has no visual meshes, still need to mark it as mesh complete
                                commands
                                    .entity(entity)
                                    .remove::<ChunkMeshingTaskComponent>();
                                chunk_manager.mark_as_mesh_complete(coord.pos, entity);
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

                        chunk_manager.mark_as_mesh_complete(coord.pos, entity);
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

/// A system that watches for newly generated chunks and promotes them to meshing if in range.
pub fn promote_newly_generated_chunks_system(
    // input
    new_data_query: Query<
        (Entity, &ChunkBlocksComponent, &ChunkCoord),
        Added<ChunkBlocksComponent>,
    >,
    camera_query: Query<(&Camera, &ChunkCoord), With<Camera3d>>,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    let mut active_camera_chunk_pos = None;
    for (camera, chunk_coord) in camera_query.iter() {
        if camera.is_active {
            active_camera_chunk_pos = Some(chunk_coord.pos);
            break;
        }
    }

    let Some(camera_chunk_pos) = active_camera_chunk_pos else {
        return;
    };

    for (entity, _, coord) in new_data_query.iter() {
        let dx = coord.pos.x - camera_chunk_pos.x;
        let dy = coord.pos.y;
        let dz = coord.pos.z - camera_chunk_pos.z;

        let is_in_range = dx.abs() <= RENDER_DISTANCE
            && (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).contains(&dy)
            && dz.abs() <= RENDER_DISTANCE;

        if is_in_range {
            trace!(target: "chunk_loading", "Newly generated chunk at {} is in range, promoting to WantsMeshing", coord.pos);
            commands
                .entity(entity)
                .insert((WantsMeshing, CheckForMeshing));
            chunk_manager.mark_as_needs_meshing(coord.pos, entity);
        } else {
            chunk_manager.mark_as_data_ready(coord.pos, entity);
        }

        // notify neighbors that they might need to re-mesh now that we have data
        for neighbor in chunk_manager.iter_neighbors(coord.pos) {
            if let ClientChunkState::NeedsMeshing { .. } | ClientChunkState::MeshComplete { .. } =
                neighbor.state
            {
                commands.entity(neighbor.entity).insert(CheckForMeshing);
            }
        }
    }
}

/// Determines chunks to promote to meshing based on the camera position and render distance.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_meshing_targets_system(
    // input
    camera_query: Query<(&Camera, &ChunkCoord), With<Camera3d>>,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    let mut active_camera_chunk_pos = None;
    for (camera, chunk_coord) in camera_query.iter() {
        if camera.is_active {
            active_camera_chunk_pos = Some(chunk_coord.pos);
            break;
        }
    }

    let Some(camera_chunk_pos) = active_camera_chunk_pos else {
        return;
    };

    let mut desired_mesh_chunks = HashSet::new();

    for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
        for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let coord = IVec3::new(camera_chunk_pos.x + x, y, camera_chunk_pos.z + z);
                desired_mesh_chunks.insert(coord);
            }
        }
    }

    for (coord, state) in chunk_manager.chunk_states.iter_mut() {
        if desired_mesh_chunks.contains(coord)
            && let ClientChunkState::DataReady { entity } = state
        {
            debug!(target:"chunk_meshing", "Promoting chunk {:?} to WantsMeshing", coord);
            commands
                .entity(*entity)
                .insert((WantsMeshing, CheckForMeshing));
            *state = ClientChunkState::NeedsMeshing { entity: *entity };
        }
    }
}

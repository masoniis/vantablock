use crate::prelude::*;
use crate::simulation::chunk::chunk_state_manager::NEIGHBOR_OFFSETS;
use crate::simulation::chunk::common::padded_chunk_view::{
    ChunkDataOption, NeighborLODs, PaddedChunk,
};
use crate::simulation::chunk::thread_buffer_pool::{acquire_buffer, release_buffer};
use crate::simulation::chunk::{
    CheckForMeshing, ChunkMeshDirty, ChunkMeshingTaskComponent, ChunkState, WantsMeshing,
    downsample_chunk, upsample_chunk,
};
use crate::simulation::{
    block::BlockRegistryResource,
    chunk::{ChunkBlocksComponent, ChunkCoord, ChunkStateManager, build_chunk_mesh},
};
use bevy::ecs::prelude::*;
use crossbeam::channel::unbounded;

/// A system that detects chunks marked as dirty and prepares them for re-meshing.
pub fn handle_dirty_chunks_system(
    // input
    dirty_chunks_query: Query<(Entity, &ChunkCoord), With<ChunkMeshDirty>>,

    // output
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkStateManager>,
) {
    for (entity, coord) in dirty_chunks_query.iter() {
        trace!(
            "Chunk {:?} at {} was marked as dirty, preparing for re-meshing.",
            entity, coord.pos
        );

        chunk_manager.mark_as_needs_meshing(coord.pos, entity);

        commands
            .entity(entity)
            .insert((WantsMeshing, CheckForMeshing))
            .remove::<ChunkMeshDirty>();
    }
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
    mut chunk_manager: ResMut<ChunkStateManager>,
    block_registry: Res<BlockRegistryResource>,
) {
    'chunk_loop: for (entity, chunk_comp, chunk_coord) in pending_chunks_query.iter_mut() {
        // check for cancellation
        match chunk_manager.get_state(chunk_coord.pos) {
            Some(ChunkState::WantsMeshing {
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

            if !ChunkStateManager::is_in_bounds(neighbor_coord) {
                return NeighborStatus::Ready(ChunkDataOption::OutOfBounds);
            }

            match chunk_manager.get_state(neighbor_coord) {
                Some(ChunkState::Loaded { entity: None }) => {
                    NeighborStatus::Ready(ChunkDataOption::Empty)
                }
                Some(state) if state.is_generated() => {
                    let entity = state.entity().expect("Expected entity for generated chunk");
                    match all_generated_chunks.get(entity) {
                        Ok(blocks) => NeighborStatus::Ready(ChunkDataOption::Generated(blocks.clone())),
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
        let coord_clone = chunk_coord.clone();

        trace!(target: "chunk_loading", "Starting meshing task for {}.", chunk_coord.pos);

        let (sender, receiver) = unbounded();
        rayon::spawn(move || {
            let buffer = acquire_buffer();

            let padded_view = PaddedChunk::new(&chunks, center_lod, original_neighbor_lods, buffer);

            let (opaque_mesh_option, transparent_mesh_option) = build_chunk_mesh(
                &coord_clone.to_string(),
                &padded_view,
                &block_registry_clone,
            );

            let used_buffer = padded_view.take_buffer();
            release_buffer(used_buffer);

            let _ = sender.send((opaque_mesh_option, transparent_mesh_option));
        });

        // update entity and manager
        commands
            .entity(entity)
            .insert(ChunkMeshingTaskComponent { receiver })
            .remove::<CheckForMeshing>()
            .remove::<WantsMeshing>();

        chunk_manager.mark_as_meshing(chunk_coord.pos, entity);
    }
}

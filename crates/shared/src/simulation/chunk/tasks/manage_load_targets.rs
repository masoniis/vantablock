use crate::prelude::*;
use crate::simulation::{
    chunk::{
        CheckForMeshing, ChunkCoord, ChunkLod, ChunkState, ChunkStateManager, LOAD_DISTANCE,
        NeedsGenerating, RENDER_DISTANCE, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK, WantsMeshing,
    },
    player::active_camera::ActiveCamera,
};
use bevy::ecs::prelude::*;
use bevy::math::IVec3;
use std::collections::HashSet;

/// Determines chunks to unload/load based on the camera position and render/loading distance.
///
/// Only needs to run when the camera has entered a new chunk.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_loading_targets_system(
    // Input
    active_camera: Res<ActiveCamera>,
    camera_query: Query<&ChunkCoord>,

    // Output
    mut chunk_manager: ResMut<ChunkStateManager>, // for marking loaded/unloaded
    mut commands: Commands,                       // for spawning chunk entities
) {
    let camera_chunk_pos = camera_query.get(active_camera.0).unwrap().pos;

    // desired chunks based on camera location for loading or meshing
    let mut desired_load_chunks = HashSet::new();
    let mut desired_mesh_chunks = HashSet::new();

    for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
        for z in -LOAD_DISTANCE..=LOAD_DISTANCE {
            for x in -LOAD_DISTANCE..=LOAD_DISTANCE {
                let coord = IVec3::new(camera_chunk_pos.x + x, y, camera_chunk_pos.z + z);
                desired_load_chunks.insert(coord);
            }
            for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let coord = IVec3::new(camera_chunk_pos.x + x, y, camera_chunk_pos.z + z);
                desired_mesh_chunks.insert(coord);
            }
        }
    }

    // INFO: --------------------------------
    //         unload/cancel chunking
    // --------------------------------------

    let mut coords_to_remove = Vec::new();
    let mut coords_to_demesh = Vec::new();

    for (coord, state) in chunk_manager.chunk_states.iter_mut() {
        // if chunk is within render distance and has data, ensure it is meshed
        if desired_mesh_chunks.contains(coord) {
            if let ChunkState::DataReady { entity } = state {
                debug!(target:"chunk_meshing", "Promoting chunk {:?} to NeedsMeshing", coord);
                commands
                    .entity(*entity)
                    .insert((WantsMeshing, CheckForMeshing));
                *state = ChunkState::WantsMeshing { entity: *entity };
            }
        } else if desired_load_chunks.contains(coord) {
            // chunk is outside render distance but still within load distance.
            // we want to demesh it but keep any other data it has.
            coords_to_demesh.push(*coord);
        } else {
            // chunk is outside load distance, unload it completely
            match state {
                ChunkState::NeedsGenerating { entity, .. }
                | ChunkState::Generating { entity }
                | ChunkState::DataReady { entity }
                | ChunkState::WantsMeshing { entity }
                | ChunkState::Meshing { entity }
                | ChunkState::Loaded {
                    entity: Some(entity),
                } => {
                    debug!(target:"chunk_loading", "Unloading chunk at {:?} (Entity: {:?})", coord, entity);
                    commands.entity(*entity).despawn();
                }
                ChunkState::Loaded { entity: None } => {
                    // already unloaded, nothing to despawn
                    debug!(target:"chunk_loading", "Marking chunk at {:?} as unloaded (was already unloaded)", coord);
                }
            }
            coords_to_remove.push(*coord);
        }
    }

    // remove the unloaded/cancelled chunks from the manager
    for coord in coords_to_remove {
        chunk_manager.mark_as_unloaded(coord);
    }

    // TODO: iterate through coords_to_demesh and handle them. Currently we don't
    // do anything with them which leaves extra meshes on the border which actually
    // could be considered a feature idk

    // INFO: --------------------------------------------
    //         load new chunks (start generation)
    // --------------------------------------------------

    // if any desired chunks are not currently loaded or loading, spawn a new chunk entity and mark it as needs-generation
    for coord in desired_load_chunks {
        if !chunk_manager.is_chunk_present_or_loading(coord) {
            debug!(target:"chunk_loading","Marking chunk needs-generation at {:?}", coord);
            let ent = commands
                .spawn((
                    NeedsGenerating { lod: ChunkLod(0) },
                    ChunkCoord { pos: coord },
                ))
                .id();
            chunk_manager.mark_as_needs_generating(coord, ent);
        }
    }
}

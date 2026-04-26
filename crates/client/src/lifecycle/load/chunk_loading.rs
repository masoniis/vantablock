use crate::prelude::*;
use bevy::ecs::prelude::*;
use bevy::math::IVec3;
use bevy::prelude::{Camera, Camera3d};
use shared::simulation::chunk::{
    ChunkCoord, ChunkState, ChunkStateManager, LOAD_DISTANCE, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
};
use std::collections::HashSet;

/// Determines chunks to unload/load based on the camera position and loading distance.
///
/// Only needs to run when the camera has entered a new chunk.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_loading_targets_system(
    // Input
    camera_query: Query<(&Camera, &ChunkCoord), With<Camera3d>>,

    // Output
    mut chunk_manager: ResMut<ChunkStateManager>, // for marking loaded/unloaded
    mut commands: Commands,                       // for spawning chunk entities
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

    // desired chunks based on camera location for loading
    let mut desired_load_chunks = HashSet::new();

    for y in WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK {
        for z in -LOAD_DISTANCE..=LOAD_DISTANCE {
            for x in -LOAD_DISTANCE..=LOAD_DISTANCE {
                let coord = IVec3::new(camera_chunk_pos.x + x, y, camera_chunk_pos.z + z);
                desired_load_chunks.insert(coord);
            }
        }
    }

    // INFO: --------------------------------
    //         unload/cancel chunking
    // --------------------------------------

    let mut coords_to_remove = Vec::new();

    for (coord, state) in chunk_manager.chunk_states.iter_mut() {
        if !desired_load_chunks.contains(coord) {
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

    // INFO: -------------------------
    //         load new chunks
    // -------------------------------

    // if any desired chunks are not currently loaded or loading, spawn a new chunk entity and mark it as needs-generation
    for coord in desired_load_chunks {
        if !chunk_manager.is_chunk_present_or_loading(coord) {
            debug!(target:"chunk_loading","Marking chunk as requested at {:?}", coord);
            let ent = commands.spawn((ChunkCoord { pos: coord },)).id();
            chunk_manager.mark_as_needs_generating(coord, ent);
        }
    }
}

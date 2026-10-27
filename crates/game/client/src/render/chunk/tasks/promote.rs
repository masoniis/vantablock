use crate::{
    prelude::*,
    render::chunk::tasks::{CheckForMeshing, WantsMeshing},
    world::chunk::{
        ClientChunkManager, ClientChunkState,
        range::{ChunkCameraQuery, active_camera_chunk, is_within_horizontal_range},
    },
};
use bevy::ecs::prelude::*;
use common::world::chunk::{ChunkBlocksComponent, ChunkCoord, RENDER_DISTANCE};

/// A system that watches for newly generated chunks and promotes them to meshing if in range.
pub fn promote_newly_generated_chunks_system(
    // input
    new_data_query: Query<
        (Entity, &ChunkBlocksComponent, &ChunkCoord),
        Added<ChunkBlocksComponent>,
    >,
    camera_query: ChunkCameraQuery,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    let Some(camera_chunk_pos) = active_camera_chunk(&camera_query) else {
        return;
    };

    for (entity, _, coord) in new_data_query.iter() {
        if is_within_horizontal_range(camera_chunk_pos, coord.pos, RENDER_DISTANCE) {
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
    camera_query: ChunkCameraQuery,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    let Some(camera_chunk_pos) = active_camera_chunk(&camera_query) else {
        return;
    };

    for (coord, state) in chunk_manager.chunk_states.iter_mut() {
        if is_within_horizontal_range(camera_chunk_pos, *coord, RENDER_DISTANCE)
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

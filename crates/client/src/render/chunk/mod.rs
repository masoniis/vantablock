pub mod asset;
pub mod components;
pub mod meshing;
pub mod tasks;

pub use asset::VoxelMeshAsset;
pub use components::{OpaqueMeshComponent, TransparentMeshComponent};
pub use shared::simulation::chunk::ChunkMeshDirty;

use crate::prelude::*;
use bevy::app::{App, FixedUpdate, Plugin, PreUpdate};
use bevy::asset::AssetApp;
use bevy::ecs::prelude::*;
use bevy::prelude::{Camera, Camera3d};
use shared::simulation::chunk::{ChunkCoord, ChunkStateManager};
use shared::simulation::scheduling::FixedUpdateSet;

pub struct ChunkMeshingPlugin;

impl Plugin for ChunkMeshingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<VoxelMeshAsset>();

        app.add_systems(
            PreUpdate,
            (manage_distance_based_chunk_meshing_targets_system).run_if(
                |q: Query<(&Camera, &ChunkCoord), (With<Camera3d>, Changed<ChunkCoord>)>| {
                    q.iter().any(|(c, _)| c.is_active)
                },
            ),
        );

        app.add_systems(PreUpdate, promote_newly_generated_chunks_system);

        app.add_systems(
            FixedUpdate,
            (
                tasks::meshgen::start_meshing::handle_dirty_chunks_system,
                tasks::meshgen::systems::start_pending_meshing_tasks_system,
                tasks::meshgen::systems::poll_chunk_meshing_tasks,
            )
                .in_set(FixedUpdateSet::MainLogic),
        );
    }
}

/// A system that watches for newly generated chunks and promotes them to meshing if in range.
pub fn promote_newly_generated_chunks_system(
    // input
    new_data_query: Query<
        (
            Entity,
            &shared::simulation::chunk::ChunkBlocksComponent,
            &ChunkCoord,
        ),
        Added<shared::simulation::chunk::ChunkBlocksComponent>,
    >,
    camera_query: Query<(&Camera, &ChunkCoord), With<Camera3d>>,

    // output
    mut chunk_manager: ResMut<ChunkStateManager>,
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

    use shared::simulation::chunk::{
        ChunkState, RENDER_DISTANCE, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
    };
    use tasks::meshgen::{CheckForMeshing, WantsMeshing};

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

        // notify neighbors
        for neighbor in chunk_manager.iter_neighbors(coord.pos) {
            if let ChunkState::WantsMeshing { .. } = neighbor.state {
                commands.entity(neighbor.entity).insert(CheckForMeshing);
            }
        }
    }
}

/// Determines chunks to promote to meshing based on the camera position and render distance.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_meshing_targets_system(
    // Input
    camera_query: Query<(&Camera, &ChunkCoord), With<Camera3d>>,

    // Output
    mut chunk_manager: ResMut<ChunkStateManager>,
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

    use crate::render::chunk::tasks::meshgen::{CheckForMeshing, WantsMeshing};
    use shared::simulation::chunk::{
        ChunkState, RENDER_DISTANCE, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK,
    };
    use std::collections::HashSet;

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
            && let ChunkState::DataReady { entity } = state
        {
            debug!(target:"chunk_meshing", "Promoting chunk {:?} to WantsMeshing", coord);
            commands
                .entity(*entity)
                .insert((WantsMeshing, CheckForMeshing));
            *state = ChunkState::WantsMeshing { entity: *entity };
        }
    }
}

pub mod asset;
pub mod components;
pub mod meshing;
pub mod tasks;

pub use asset::BlockMeshAsset;
pub use common::world::chunk::ChunkMeshDirty;
pub use components::{OpaqueMeshComponent, TransparentMeshComponent};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    render::chunk::tasks::{
        handle_dirty_chunks_system, manage_distance_based_chunk_meshing_targets_system,
        poll_chunk_meshing_tasks, promote_newly_generated_chunks_system,
        start_pending_meshing_tasks_system,
    },
    world::chunk::range::active_camera_changed_chunk,
};
use bevy::{
    app::{App, FixedUpdate, Plugin, PreUpdate},
    asset::AssetApp,
    ecs::prelude::*,
};
use common::FixedUpdateSet;

/// Meshes the chunks `world::chunk` holds and hands the meshes to the render world.
pub struct ChunkMeshingPlugin;

impl Plugin for ChunkMeshingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BlockMeshAsset>();

        app.add_systems(
            PreUpdate,
            manage_distance_based_chunk_meshing_targets_system.run_if(active_camera_changed_chunk),
        );

        app.add_systems(PreUpdate, promote_newly_generated_chunks_system);

        app.add_systems(
            FixedUpdate,
            (
                handle_dirty_chunks_system,
                start_pending_meshing_tasks_system,
                poll_chunk_meshing_tasks,
            )
                .in_set(FixedUpdateSet::MainLogic),
        );
    }
}

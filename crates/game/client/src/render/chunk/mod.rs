pub mod asset;
pub mod components;
pub mod meshing;
pub mod tasks;

pub use asset::BlockMeshAsset;
pub use common::world::chunk::ChunkMeshDirty;
pub use components::{ChunkMesh, OpaqueMeshComponent, TransparentMeshComponent};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    lifecycle::ClientLifecycleState,
    render::chunk::tasks::{
        handle_dirty_chunks_system, poll_chunk_meshing_tasks, schedule_chunk_meshing_system,
    },
    world::chunk::decompression::apply_decompressed_chunk_data_system,
};
use bevy::{
    app::{App, Plugin, Update},
    asset::AssetApp,
    ecs::prelude::*,
    state::condition::in_state,
};

/// Meshes the chunks `world::chunk` holds and hands the meshes to the render world.
pub struct ChunkMeshingPlugin;

impl Plugin for ChunkMeshingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BlockMeshAsset>();

        // polling before scheduling lets slots freed this frame be refilled this frame. Only in
        // game: the block render registry it meshes with is loaded while launching.
        app.add_systems(
            Update,
            (
                handle_dirty_chunks_system,
                poll_chunk_meshing_tasks,
                schedule_chunk_meshing_system,
            )
                .chain()
                .after(apply_decompressed_chunk_data_system)
                .run_if(in_state(ClientLifecycleState::InGame)),
        );
    }
}

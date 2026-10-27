pub mod asset;
pub mod components;
pub mod manager;
pub mod meshing;
pub mod tasks;

pub use asset::BlockMeshAsset;
pub use common::world::chunk::ChunkMeshDirty;
pub use components::{OpaqueMeshComponent, TransparentMeshComponent};
pub use manager::{ClientChunkManager, ClientChunkState};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::chunk::tasks::{
    apply_decompressed_chunk_data_system, decompress_chunk_data_system,
    decompression::DecompressedChunkMessage, handle_dirty_chunks_system,
    manage_distance_based_chunk_meshing_targets_system, poll_chunk_meshing_tasks,
    poll_decompression_tasks_system, promote_newly_generated_chunks_system,
    start_pending_meshing_tasks_system,
};
use bevy::{
    app::{App, FixedUpdate, Plugin, PreUpdate, Update},
    asset::AssetApp,
    ecs::prelude::*,
    prelude::{Camera, Camera3d},
};
use common::{FixedUpdateSet, world::chunk::ChunkCoord};

pub struct ChunkMeshingPlugin;

impl Plugin for ChunkMeshingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BlockMeshAsset>();
        app.init_resource::<ClientChunkManager>();

        app.add_message::<DecompressedChunkMessage>();

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
            Update,
            (
                decompress_chunk_data_system,
                poll_decompression_tasks_system,
            ),
        );

        app.add_systems(
            FixedUpdate,
            (
                apply_decompressed_chunk_data_system,
                handle_dirty_chunks_system,
                start_pending_meshing_tasks_system,
                poll_chunk_meshing_tasks,
            )
                .in_set(FixedUpdateSet::MainLogic),
        );
    }
}

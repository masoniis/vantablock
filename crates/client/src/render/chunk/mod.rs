pub mod asset;
pub mod components;
pub mod manager;
pub mod meshing;
pub mod tasks;

pub use asset::BlockMeshAsset;
pub use components::{OpaqueMeshComponent, TransparentMeshComponent};
pub use manager::{ClientChunkManager, ClientChunkState};
pub use shared::world::chunk::ChunkMeshDirty;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::chunk::tasks::systems::{
    manage_distance_based_chunk_meshing_targets_system, promote_newly_generated_chunks_system,
};
use bevy::{
    app::{App, FixedUpdate, Plugin, PreUpdate},
    asset::AssetApp,
    ecs::prelude::*,
    prelude::{Camera, Camera3d},
};
use shared::{FixedUpdateSet, world::chunk::ChunkCoord};

pub struct ChunkMeshingPlugin;

impl Plugin for ChunkMeshingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BlockMeshAsset>();
        app.init_resource::<ClientChunkManager>();

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
                tasks::start_meshing::handle_dirty_chunks_system,
                tasks::systems::start_pending_meshing_tasks_system,
                tasks::systems::poll_chunk_meshing_tasks,
            )
                .in_set(FixedUpdateSet::MainLogic),
        );
    }
}

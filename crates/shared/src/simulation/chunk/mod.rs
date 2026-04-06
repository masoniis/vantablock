pub mod common;
pub mod components;
pub mod consts;
pub mod meshing;
pub mod tasks;
pub mod types;

pub use common::*;
pub use components::*;
pub use consts::*;
pub use meshing::*;
pub use tasks::*;
pub use types::*;

// INFO: ------------------------------
//         chunk loading plugin
// ------------------------------------

use crate::simulation::{player::active_camera::ActiveCamera, scheduling::FixedUpdateSet};
use bevy::app::{App, FixedUpdate, Plugin, PreUpdate};
use bevy::ecs::prelude::*;

pub struct ChunkLoadingPlugin;

impl Plugin for ChunkLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkStateManager::default());

        app.add_systems(
            PreUpdate,
            (manage_distance_based_chunk_loading_targets_system).run_if(
                |camera: Res<ActiveCamera>, q: Query<(), Changed<ChunkCoord>>| {
                    q.get(camera.0).is_ok()
                },
            ),
        );

        app.add_systems(
            FixedUpdate,
            (
                handle_dirty_chunks_system,
                start_pending_generation_tasks_system,
                poll_chunk_generation_tasks,
                start_pending_meshing_tasks_system,
                poll_chunk_meshing_tasks,
            )
                .in_set(FixedUpdateSet::MainLogic),
        );
    }
}

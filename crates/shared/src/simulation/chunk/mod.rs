pub mod common;
pub mod components;
pub mod consts;
pub mod tasks;
pub mod types;

pub use common::*;
pub use components::*;
pub use consts::*;
pub use tasks::*;
pub use types::*;

// INFO: ------------------------------
//         chunk loading plugin
// ------------------------------------

use crate::simulation::scheduling::FixedUpdateSet;
use bevy::app::{App, FixedUpdate, Plugin, PreUpdate};
use bevy::ecs::prelude::*;
use bevy::prelude::{Camera, Camera3d};

pub struct ChunkLoadingPlugin;

impl Plugin for ChunkLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkStateManager::default());

        app.add_systems(
            PreUpdate,
            (manage_distance_based_chunk_loading_targets_system).run_if(
                |q: Query<(&Camera, &ChunkCoord), (With<Camera3d>, Changed<ChunkCoord>)>| {
                    q.iter().any(|(c, _)| c.is_active)
                },
            ),
        );

        app.add_systems(
            FixedUpdate,
            (
                start_pending_generation_tasks_system,
                poll_chunk_generation_tasks,
            )
                .in_set(FixedUpdateSet::MainLogic),
        );
    }
}

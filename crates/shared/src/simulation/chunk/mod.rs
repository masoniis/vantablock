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

use crate::FixedUpdateSet;
use bevy::app::{App, FixedUpdate, Plugin};
use bevy::ecs::prelude::*;

pub struct ChunkLoadingPlugin;

impl Plugin for ChunkLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkStateManager::default());

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

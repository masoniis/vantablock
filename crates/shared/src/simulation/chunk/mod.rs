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

use bevy::app::{App, Plugin};

pub struct ChunkLoadingPlugin;

impl Plugin for ChunkLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkStateManager::default());
    }
}

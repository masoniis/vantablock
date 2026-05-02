pub mod common;
pub mod components;
pub mod consts;
mod systems;
pub mod types;

pub use common::*;
pub use components::*;
pub use consts::*;
pub use types::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin, Update};
use systems::update_chunk_coords_system;

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_chunk_coords_system);
    }
}

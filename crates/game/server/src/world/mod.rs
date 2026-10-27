pub mod chunk;
pub mod terrain;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;
use terrain::TerrainGenerationPlugin;

pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((TerrainGenerationPlugin, chunk::ChunkPlugin));
    }
}

pub mod chunk;
pub mod clock;
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

        // the clock ticks in `FixedUpdate`, so read the keys on the same tick as the inputs
        app.add_message::<clock::ClockAdjustedMessage>();
        app.add_systems(
            FixedUpdate,
            (
                clock::apply_time_control_inputs_system,
                clock::apply_set_time_of_day_requests_system,
            ),
        );
    }
}

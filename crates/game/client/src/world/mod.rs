pub mod chunk;
pub mod clock;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;

/// Client-side world state that is independent of rendering: the chunks it holds and the world
/// clock it follows.
pub struct ClientWorldPlugin;

impl Plugin for ClientWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(chunk::ClientChunkPlugin);
        app.add_systems(Update, clock::follow_server_clock_system);
    }
}

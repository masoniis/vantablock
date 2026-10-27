pub mod chunk;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::prelude::*;

/// Client-side world state that is independent of rendering.
pub struct ClientWorldPlugin;

impl Plugin for ClientWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(chunk::ClientChunkPlugin);
    }
}

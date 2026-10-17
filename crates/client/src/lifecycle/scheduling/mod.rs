pub mod sets;

pub use sets::RenderPrepSet;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::state::ClientLifecycleState;
use bevy::prelude::*;

pub struct ClientSchedulingPlugin;

impl Plugin for ClientSchedulingPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            RenderPrepSet.run_if(in_state(ClientLifecycleState::InGame)),
        );
    }
}

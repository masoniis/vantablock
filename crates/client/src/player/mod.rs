pub mod camera;
pub mod components;
pub mod replicated_player;
mod systems;

pub use camera::*;
pub use components::*;

// INFO: -----------------------
//         player plugin
// -----------------------------

use crate::lifecycle::ClientLifecycleState;
use bevy::prelude::*;
use lightyear::prelude::client::input::InputSystems;
use systems::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CameraPlugin);

        app.add_systems(
            FixedPreUpdate,
            look::write_look_input_system
                .in_set(InputSystems::WriteClientInputs)
                .run_if(in_state(ClientLifecycleState::InGame)),
        );

        app.add_systems(
            Update,
            (
                setup_replicated_players::dress_replicated_players_system,
                look::local_player_look_system,
            )
                .run_if(in_state(ClientLifecycleState::InGame)),
        );
    }
}

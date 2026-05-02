pub mod camera;
pub mod components;
pub mod events;
pub mod replicated_player;
pub mod systems;
pub mod targeted_block;

pub use camera::*;
pub use components::*;
pub use events::*;
pub use systems::*;
pub use targeted_block::TargetedBlock;

// INFO: -----------------------
//         player plugin
// -----------------------------

use crate::player::replicated_player::dress_predicted_player_observer;
use bevy::app::{App, FixedUpdate, Plugin, Update};
use bevy::ecs::message::Messages;
use bevy::ecs::schedule::IntoScheduleConfigs;
use leafwing_input_manager::common_conditions::action_just_pressed;
use shared::player::PlayerAction;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>();

        app.add_plugins(CameraPlugin);

        app.add_systems(
            FixedUpdate,
            update_target_block::update_targeted_block_system,
        );

        app.add_observer(dress_predicted_player_observer);

        // register local block events
        app.init_resource::<Messages<BreakBlockEvent>>();
        app.init_resource::<Messages<PlaceBlockEvent>>();

        app.add_systems(
            Update,
            (
                block_actions::break_targeted_block_system
                    .run_if(action_just_pressed(PlayerAction::BreakBlock)),
                block_actions::place_targeted_block_system
                    .run_if(action_just_pressed(PlayerAction::PlaceBlock)),
                block_actions::handle_break_block_events_system,
                block_actions::handle_place_block_events_system,
                block_actions::handle_incoming_block_updates,
            ),
        );
    }
}

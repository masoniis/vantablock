pub mod camera;
pub mod components;
pub mod events;
pub mod replicated_player;
mod systems;
pub mod targeted_block;

pub use camera::*;
pub use components::*;
pub use events::*;
pub use targeted_block::TargetedBlock;

// INFO: -----------------------
//         player plugin
// -----------------------------

use crate::{
    lifecycle::{ClientLifecycleState, state::InGameState},
    network::send_block_action_messages_system,
};
use bevy::prelude::*;
use common::player::PlayerAction;
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::client::input::InputSystems;
use systems::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>();

        app.add_plugins(CameraPlugin);

        // register local block events
        app.add_message::<BreakBlockEvent>();
        app.add_message::<PlaceBlockEvent>();

        app.add_systems(
            PostUpdate,
            update_target_block::update_targeted_block_system
                .run_if(in_state(ClientLifecycleState::InGame)),
        );

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
                block_actions::break_targeted_block_system
                    .before(send_block_action_messages_system)
                    .run_if(local_action_just_pressed(PlayerAction::PrimaryInteract)),
                block_actions::place_targeted_block_system
                    .before(send_block_action_messages_system)
                    .run_if(local_action_just_pressed(PlayerAction::SecondaryInteract)),
                block_actions::handle_break_block_events_system,
                block_actions::handle_place_block_events_system,
                block_actions::handle_incoming_block_updates,
            )
                .run_if(in_state(ClientLifecycleState::InGame)),
        );

        // mouse look only while playing, so the pause menu cursor doesn't turn the camera
        app.add_systems(
            Update,
            look::local_player_look_system.run_if(in_state(InGameState::Playing)),
        );
    }
}

/// Run condition for a `PlayerAction` just pressed by the local player.
///
/// Leafwing's `action_just_pressed` reads any `Single` `ActionState<PlayerAction>`, but player
/// input is networked per player entity, so this narrows it to the local player's.
fn local_action_just_pressed(
    action: PlayerAction,
) -> impl Fn(Query<&ActionState<PlayerAction>, With<LocalPlayer>>) -> bool {
    move |query: Query<&ActionState<PlayerAction>, With<LocalPlayer>>| {
        query
            .single()
            .is_ok_and(|action_state| action_state.just_pressed(&action))
    }
}

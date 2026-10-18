mod time_requests;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{input::local_actions::ClientAction, lifecycle::ClientLifecycleState};
use bevy::{
    app::{App, Plugin, Update},
    prelude::{IntoScheduleConfigs, SystemCondition, in_state},
};
use leafwing_input_manager::common_conditions::action_just_pressed;

pub struct NetworkSendPlugin;

impl Plugin for NetworkSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            time_requests::request_clock_changes_system
                .run_if(in_state(ClientLifecycleState::InGame))
                .run_if(
                    action_just_pressed(ClientAction::TimeForward)
                        .or(action_just_pressed(ClientAction::TimeBackward))
                        .or(action_just_pressed(ClientAction::ToggleDayCycle)),
                ),
        );
    }
}

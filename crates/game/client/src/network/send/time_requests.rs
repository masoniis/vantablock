use crate::input::local_actions::ClientAction;
use bevy::prelude::*;
use common::network::{ChatAndSystem, ClientMessage};
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::{Client, MessageSender};

/// How far one press of the time keys moves the time of day.
const TIME_JUMP_SECONDS: f32 = 20.0;

/// Asks the server to change the world clock. The change arrives back through clock syncs.
pub fn request_clock_changes_system(
    actions: Res<ActionState<ClientAction>>,
    mut sender_query: Query<&mut MessageSender<ClientMessage>, With<Client>>,
) {
    let Ok(mut sender) = sender_query.single_mut() else {
        return;
    };
    if actions.just_pressed(&ClientAction::TimeForward) {
        sender.send::<ChatAndSystem>(ClientMessage::RequestTimeJump {
            seconds: TIME_JUMP_SECONDS,
        });
    }
    if actions.just_pressed(&ClientAction::TimeBackward) {
        sender.send::<ChatAndSystem>(ClientMessage::RequestTimeJump {
            seconds: -TIME_JUMP_SECONDS,
        });
    }
    if actions.just_pressed(&ClientAction::ToggleDayCycle) {
        sender.send::<ChatAndSystem>(ClientMessage::RequestToggleDayCycle);
    }
}

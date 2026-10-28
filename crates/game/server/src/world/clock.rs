//! The authoritative world clock: players change it with the time-control keys (and the
//! showcase menu), and every client is kept in sync with it.

use crate::network::receive::ecs_messages::InboundSetTimeOfDayMessage;
use bevy::{platform::collections::HashMap, prelude::*};
use common::{
    player::{PlayerAction, components::NetworkPlayer},
    time::{JUMP_CLOCK_DISTANCE, WorldClockResource},
};
use leafwing_input_manager::prelude::ActionState;

/// Written whenever the clock is changed by hand, so clients are synced right away rather than
/// at the next periodic sync.
#[derive(Message, Clone, Copy)]
pub struct ClockAdjustedMessage;

/// The time-control keys each player held last tick, to act once per press.
#[derive(Default)]
pub struct HeldTimeKeys(HashMap<Entity, [bool; 3]>);

const TIME_KEYS: [PlayerAction; 3] = [
    PlayerAction::JumpGameTimeForward,
    PlayerAction::JumpGameTimeBackward,
    PlayerAction::PauseGameTime,
];

/// Applies the time-control keys from every player's networked inputs.
///
/// Presses are detected against the previous tick's inputs instead of `just_pressed`, which
/// depends on how the input buffer is replayed into the action state.
pub fn apply_time_control_inputs_system(
    // input
    players: Query<(Entity, &ActionState<PlayerAction>), With<NetworkPlayer>>,

    // output
    mut held: Local<HeldTimeKeys>,
    mut world_clock: ResMut<WorldClockResource>,
    mut adjusted: MessageWriter<ClockAdjustedMessage>,
) {
    held.0.retain(|entity, _| players.contains(*entity));

    for (player, actions) in &players {
        let now = TIME_KEYS.map(|key| actions.pressed(&key));
        let before = held.0.insert(player, now).unwrap_or_default();
        let pressed = |i: usize| now[i] && !before[i];

        if pressed(0) {
            world_clock.jump(JUMP_CLOCK_DISTANCE);
        }
        if pressed(1) {
            world_clock.jump(-JUMP_CLOCK_DISTANCE);
        }
        if pressed(2) {
            world_clock.paused = !world_clock.paused;
        }
        if (0..TIME_KEYS.len()).any(pressed) {
            adjusted.write(ClockAdjustedMessage);
        }
    }
}

/// Applies time-of-day requests from clients (the showcase menu).
pub fn apply_set_time_of_day_requests_system(
    mut requests: MessageReader<InboundSetTimeOfDayMessage>,
    mut world_clock: ResMut<WorldClockResource>,
    mut adjusted: MessageWriter<ClockAdjustedMessage>,
) {
    for request in requests.read() {
        world_clock.set_time_of_day(request.fraction);
        adjusted.write(ClockAdjustedMessage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_press_applies_once() {
        let mut app = App::new();
        app.init_resource::<WorldClockResource>();
        app.add_message::<ClockAdjustedMessage>();
        app.add_systems(Update, apply_time_control_inputs_system);
        let player = app
            .world_mut()
            .spawn((NetworkPlayer, ActionState::<PlayerAction>::default()))
            .id();
        let set_pause = |app: &mut App, down: bool| {
            let mut actions = app
                .world_mut()
                .get_mut::<ActionState<PlayerAction>>(player)
                .unwrap();
            if down {
                actions.press(&PlayerAction::PauseGameTime);
            } else {
                actions.release(&PlayerAction::PauseGameTime);
            }
        };

        // held for two ticks: toggles once
        set_pause(&mut app, true);
        app.update();
        app.update();
        assert!(app.world().resource::<WorldClockResource>().paused);

        // released, then pressed again: toggles back
        set_pause(&mut app, false);
        app.update();
        set_pause(&mut app, true);
        app.update();
        assert!(!app.world().resource::<WorldClockResource>().paused);
    }
}

use crate::input::resources::CursorMovement;
use crate::player::components::{LocalPlayer, LocalPlayerLook};
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use shared::player::PlayerAction;

/// A system that runs every frame on the client to update the local player's look
/// orientation based on mouse movement.
///
/// Only the client-side `LocalPlayerLook` is touched here so the camera can turn at full
/// framerate. The predicted `PlayerLook` is left to the shared fixed-step simulation, since
/// writing it outside of a tick would diverge from the prediction history and trigger
/// constant rollbacks.
pub fn local_player_look_system(
    movement: Res<CursorMovement>,
    mut query: Query<&mut LocalPlayerLook, With<LocalPlayer>>,
) {
    let Ok(mut local_look) = query.single_mut() else {
        return;
    };

    // sensitivity could be moved to a resource later
    const MOUSE_SENSITIVITY: f32 = 0.002;

    // update the absolute angles based on mouse delta
    local_look.yaw -= movement.delta.x * MOUSE_SENSITIVITY;
    local_look.pitch -= movement.delta.y * MOUSE_SENSITIVITY;

    // clamp pitch to avoid flipping the camera
    local_look.pitch = local_look
        .pitch
        .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
}

/// A system that injects the local player's absolute look angles into the tick-synced
/// `ActionState` so they are buffered and sent to the server alongside the other inputs.
///
/// Must run in `InputSystems::WriteClientInputs` (`FixedPreUpdate`) so that lightyear
/// buffers this tick's value instead of the previous frame's.
pub fn write_look_input_system(
    mut query: Query<(&mut ActionState<PlayerAction>, &LocalPlayerLook), With<LocalPlayer>>,
) {
    let Ok((mut action_state, local_look)) = query.single_mut() else {
        return;
    };

    action_state.set_axis_pair(
        &PlayerAction::Look,
        Vec2::new(local_look.yaw, local_look.pitch),
    );
}

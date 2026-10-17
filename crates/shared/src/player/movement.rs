use crate::player::{
    PlayerAction,
    components::{LogicalPosition, NetworkPlayer, PlayerLook},
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// A movement system shared by the client and server that updates the `LogicalPosition`
/// of a player based on their `PlayerAction` and `PlayerLook` states.
///
/// The server uses this to authoritatively move the networked player entity, while the
/// client uses it to make movement predictions.
pub fn shared_player_movement_system(
    time: Res<Time<Fixed>>,
    mut query: Query<
        (
            &ActionState<PlayerAction>,
            &mut PlayerLook,
            &mut LogicalPosition,
        ),
        With<NetworkPlayer>,
    >,
) {
    let delta = time.delta_secs();
    let move_speed = 20.0;

    for (action_state, mut look, mut position) in query.iter_mut() {
        let aim_data = action_state.axis_pair(&PlayerAction::Look);
        let yaw = aim_data.x;
        let pitch = aim_data.y;

        // Sync the PlayerLook component with the tick-synced ActionState
        // so it can be replicated to other clients.
        look.yaw = yaw;
        look.pitch = pitch;

        let mut move_dir = Vec3::ZERO;

        // calculate front vector based on yaw and pitch (for vertical movement)
        let forward = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0) * -Vec3::Z;
        // calculate right vector based on yaw only (for horizontal strafing)
        let right = Quat::from_rotation_y(yaw) * Vec3::X;

        if action_state.pressed(&PlayerAction::MoveForward) {
            move_dir += forward;
        }
        if action_state.pressed(&PlayerAction::MoveBackward) {
            move_dir -= forward;
        }
        if action_state.pressed(&PlayerAction::MoveLeft) {
            move_dir -= right;
        }
        if action_state.pressed(&PlayerAction::MoveRight) {
            move_dir += right;
        }

        if move_dir != Vec3::ZERO {
            move_dir = move_dir.normalize();
            position.0 += move_dir * move_speed * delta;
        }
    }
}

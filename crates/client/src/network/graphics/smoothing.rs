use crate::player::LocalPlayerLook;
use bevy::prelude::*;
use shared::player::components::{LogicalPosition, PlayerLook};

/// Smoothly interpolates the visual Transform to match the LogicalPosition.
pub fn update_logical_position_smoothing(
    time: Res<Time>,
    mut query: Query<(&LogicalPosition, &mut Transform)>,
) {
    // stiff spring for snappy but smooth camera follow
    let spring_stiffness = 25.0;

    for (logical_pos, mut transform) in query.iter_mut() {
        transform.translation = transform
            .translation
            .lerp(logical_pos.0, time.delta_secs() * spring_stiffness);
    }
}

/// Snaps the visual Transform rotation to match the player's look.
///
/// The local player uses its per-frame `LocalPlayerLook` so the camera never waits on the
/// fixed tick, while remote players use their interpolated `PlayerLook`.
pub fn update_player_look_smoothing(
    mut query: Query<(&PlayerLook, Option<&LocalPlayerLook>, &mut Transform)>,
) {
    for (look, local_look, mut transform) in query.iter_mut() {
        let (yaw, pitch) = local_look.map_or((look.yaw, look.pitch), |l| (l.yaw, l.pitch));
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

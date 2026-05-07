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

/// Snaps the visual Transform rotation to match the PlayerLook.
pub fn update_player_look_smoothing(mut query: Query<(&PlayerLook, &mut Transform)>) {
    for (look, mut transform) in query.iter_mut() {
        transform.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
    }
}

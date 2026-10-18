use crate::player::LocalPlayerLook;
use bevy::prelude::*;
use shared::player::components::{LogicalPosition, PlayerLook};

/// How quickly the visual position closes the gap to the logical one, per second.
const POSITION_SMOOTHING_RATE: f32 = 25.0;

/// Smoothly interpolates the visual Transform to match the LogicalPosition.
///
/// Uses exponential decay, which is frame-rate independent and never overshoots. A plain
/// `lerp(target, dt * rate)` overshoots once a frame takes longer than `2 / rate` seconds and
/// then diverges, flinging the camera toward infinity during slow frames.
pub fn update_logical_position_smoothing(
    time: Res<Time>,
    mut query: Query<(&LogicalPosition, &mut Transform)>,
) {
    let blend = smoothing_blend(time.delta_secs(), POSITION_SMOOTHING_RATE);
    for (logical_pos, mut transform) in query.iter_mut() {
        transform.translation = transform.translation.lerp(logical_pos.0, blend);
    }
}

/// The fraction of the remaining gap to close this frame, always in `0..=1`.
fn smoothing_blend(delta_secs: f32, rate: f32) -> f32 {
    1.0 - (-rate * delta_secs).exp()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: slow frames pushed the blend factor above 2, so the camera overshot further
    /// every frame and diverged.
    #[test]
    fn smoothing_converges_even_with_slow_frames() {
        for delta in [0.001, 1.0 / 60.0, 0.1, 0.5, 5.0] {
            let blend = smoothing_blend(delta, POSITION_SMOOTHING_RATE);
            assert!(
                (0.0..=1.0).contains(&blend),
                "dt {delta} gave blend {blend}"
            );

            let mut position = Vec3::new(100.0, -50.0, 3.0);
            for _ in 0..200 {
                position = position.lerp(Vec3::ZERO, blend);
            }
            assert!(
                position.length() < 100.0,
                "dt {delta} diverged to {position}"
            );
        }
    }
}

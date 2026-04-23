use crate::input::resources::CursorMovement;
use crate::prelude::*;
use bevy::ecs::prelude::{MessageReader, ResMut};
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use tracing::instrument;

/// A system to handle Bevy's native mouse motion and wheel events,
/// and simultaneously update the input resource with device information.
#[instrument(skip_all)]
pub fn device_events_system(
    mut motion_events: MessageReader<MouseMotion>,
    mut wheel_events: MessageReader<MouseWheel>,
    // State to modify (output)
    mut movement: ResMut<CursorMovement>,
) {
    // Clear previous stale state (without this mouse movement would "accumulate")
    movement.reset_deltas();

    for event in motion_events.read() {
        movement.adjust_mouse_delta(event.delta.into());
    }

    for event in wheel_events.read() {
        let y_offset = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y,
        };
        movement.adjust_scroll_delta(Vec2::new(0.0, y_offset));
    }
}

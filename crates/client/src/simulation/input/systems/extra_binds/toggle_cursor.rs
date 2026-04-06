use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

/// Toggles the OS cursor visibility and lock state on the primary window.
pub fn toggle_cursor_system(mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor_query.single_mut() else {
        return;
    };

    // toggle visibiility
    cursor.visible = !cursor.visible;

    if cursor.visible {
        cursor.grab_mode = CursorGrabMode::None;
    } else {
        cursor.grab_mode = CursorGrabMode::Locked;
    }
}

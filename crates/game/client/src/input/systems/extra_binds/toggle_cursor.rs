use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

/// Locks the OS cursor and hides it.
pub fn lock_cursor_system(mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor_query.single_mut() else {
        return;
    };

    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Locked;
}

/// Unlocks the OS cursor and shows it.
pub fn unlock_cursor_system(mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor_query.single_mut() else {
        return;
    };

    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}

//! Styled widgets built on Bevy's headless `ui_widgets`, as `bsn!` scene functions.
//!
//! The headless widgets own behavior (pressing, focus, editing, the `Activate` event); the systems
//! here only restyle them from their state, so screens attach behavior with `on(...)` observers.

mod button;
mod keycap;
mod spinner;
mod text_field;

pub use button::{ButtonTone, button};
pub use keycap::{key_label, keycap, keycaps};
pub use spinner::spinner;
pub use text_field::text_field;

use bevy::{input_focus::tab_navigation::TabNavigationPlugin, prelude::*};

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        app.add_systems(
            PostUpdate,
            (
                button::style_buttons,
                text_field::style_text_fields,
                spinner::animate_spinners,
            ),
        );
    }
}

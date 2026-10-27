pub mod connecting;
pub mod debug_menu;
pub mod disconnected;
mod launching_client;
pub mod main_menu;
pub mod pause_menu;

pub use launching_client::LaunchingClientScreenPlugin;

use crate::ui::{
    theme::{Face, palette, panel, text},
    widgets::spinner,
};
use bevy::{prelude::*, text::LetterSpacing};

/// A small card for transient states: a title, a line of detail, and a spinner.
fn status_card(title: &str, detail: &str) -> impl Scene + use<> {
    let title = title.to_uppercase();
    let detail = detail.to_owned();
    bsn! {
        panel(palette::ACCENT)
        Node { width: px(360), align_items: AlignItems::Center, row_gap: px(14) }
        Children [
            (
                text(&title, Face::Display, 22.0, palette::TEXT)
                template_value(LetterSpacing::Px(3.0))
            ),
            text(&detail, Face::Sans, 14.0, palette::TEXT_DIM),
            spinner(),
        ]
    }
}

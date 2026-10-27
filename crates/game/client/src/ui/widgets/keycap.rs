use crate::ui::theme::{Face, palette, text};
use bevy::prelude::*;
use std::borrow::Cow;

/// A key drawn as a small keycap, with a thicker bottom edge so it reads as a physical key.
pub fn keycap(label: &str) -> impl Scene + use<> {
    let label = label.to_owned();
    bsn! {
        Node {
            min_width: px(26),
            height: px(24),
            padding: UiRect::horizontal(px(7)),
            border: UiRect { left: px(1), right: px(1), top: px(1), bottom: px(3) },
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor(palette::SURFACE)
        BorderColor::all(palette::BORDER)
        Children [ text(&label, Face::Mono, 12.0, palette::TEXT) ]
    }
}

/// A row of keycaps, one per key.
pub fn keycaps(keys: &[KeyCode]) -> impl Scene + use<> {
    let caps: Vec<_> = keys.iter().map(|&key| keycap(&key_label(key))).collect();
    bsn! {
        Node { column_gap: px(4), align_items: AlignItems::Center }
        Children [ {caps} ]
    }
}

/// The short label printed on a key's cap.
pub fn key_label(key: KeyCode) -> Cow<'static, str> {
    let label = match key {
        KeyCode::Escape => "Esc",
        KeyCode::Space => "Space",
        KeyCode::Enter => "Enter",
        KeyCode::Tab => "Tab",
        KeyCode::Backspace => "Bksp",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl",
        KeyCode::AltLeft | KeyCode::AltRight => "Alt",
        KeyCode::SuperLeft | KeyCode::SuperRight => "Cmd",
        KeyCode::ArrowUp => "↑",
        KeyCode::ArrowDown => "↓",
        KeyCode::ArrowLeft => "←",
        KeyCode::ArrowRight => "→",
        _ => {
            // `KeyW` -> `W`, `Digit1` -> `1`, `F3` -> `F3`
            let name = format!("{key:?}");
            let short = name
                .strip_prefix("Key")
                .or_else(|| name.strip_prefix("Digit"))
                .unwrap_or(&name);
            return Cow::Owned(short.to_owned());
        }
    };
    Cow::Borrowed(label)
}

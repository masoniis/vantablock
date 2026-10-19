//! The shared look of the client UI: one palette, one variable font, and the text scenes every
//! screen is built from.
//!
//! The look is deliberately flat and blocky: square corners, hard 2px borders, a near-black
//! backdrop, and a single mint accent for whatever is interactive. All text uses Recursive, whose
//! variable axes give a heavy display face, a regular sans, and a monospace face (for numbers,
//! keys, and the debug overlay) from one asset.

use bevy::{
    prelude::*,
    text::{FontSourceTemplate, FontVariationTag, FontVariations, FontWeight, LetterSpacing},
};

pub const FONT: &str = "client/font/Recursive_variable.ttf";

pub mod palette {
    use bevy::color::Color;

    // surfaces
    pub const BACKDROP_TOP: Color = Color::srgb(0.06, 0.07, 0.09);
    pub const BACKDROP_BOTTOM: Color = Color::srgb(0.012, 0.014, 0.02);
    pub const SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
    pub const PANEL: Color = Color::srgba(0.03, 0.035, 0.045, 0.94);
    pub const PANEL_BORDER: Color = Color::srgb(0.15, 0.17, 0.2);
    pub const SURFACE: Color = Color::srgb(0.08, 0.09, 0.11);
    pub const SURFACE_HOVER: Color = Color::srgb(0.12, 0.135, 0.165);
    pub const FIELD: Color = Color::srgb(0.015, 0.017, 0.022);
    pub const BORDER: Color = Color::srgb(0.22, 0.25, 0.3);
    pub const BORDER_HOVER: Color = Color::srgb(0.38, 0.42, 0.48);

    // text
    pub const TEXT: Color = Color::srgb(0.93, 0.94, 0.96);
    pub const TEXT_DIM: Color = Color::srgb(0.56, 0.59, 0.65);
    pub const TEXT_FAINT: Color = Color::srgb(0.34, 0.37, 0.42);

    // signal
    pub const ACCENT: Color = Color::srgb(0.36, 0.9, 0.68);
    pub const ACCENT_DEEP: Color = Color::srgb(0.1, 0.32, 0.24);
    pub const DANGER: Color = Color::srgb(1.0, 0.38, 0.36);
    pub const DANGER_DEEP: Color = Color::srgb(0.3, 0.07, 0.07);

    // debug overlay, in the spirit of the colored F3 mods
    pub const OVERLAY_LINE: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
    pub const LABEL: Color = Color::srgb(1.0, 0.8, 0.32);
    pub const HEADING: Color = Color::srgb(0.36, 0.9, 0.68);
    pub const ENUM: Color = Color::srgb(0.45, 0.78, 1.0);
    pub const GOOD: Color = Color::srgb(0.45, 0.92, 0.45);
    pub const WARN: Color = Color::srgb(1.0, 0.75, 0.3);
    pub const BAD: Color = Color::srgb(1.0, 0.38, 0.36);
    pub const AXIS_X: Color = Color::srgb(1.0, 0.45, 0.42);
    pub const AXIS_Y: Color = Color::srgb(0.5, 0.92, 0.45);
    pub const AXIS_Z: Color = Color::srgb(0.42, 0.65, 1.0);
}

/// One of the faces cut from the variable font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// Heavy, for titles.
    Display,
    /// Regular proportional text.
    Sans,
    /// Semibold proportional text, for button captions and headings.
    Strong,
    /// Monospace, for numbers, addresses, keys, and the debug overlay.
    Mono,
}

/// A `TextFont` in the given face and size.
pub fn font(face: Face, size: f32) -> impl Scene {
    let (weight, mono): (u16, f32) = match face {
        Face::Display => (900, 0.0),
        Face::Sans => (400, 0.0),
        Face::Strong => (640, 0.0),
        Face::Mono => (480, 1.0),
    };
    let variations = FontVariations::builder()
        .set(FontVariationTag::new(b"MONO"), mono)
        .build();
    bsn! {
        TextFont {
            font: FontSourceTemplate::Handle(FONT),
            font_size: FontSize::Px(size),
            weight: FontWeight(weight),
            font_variations: variations,
        }
    }
}

/// A text entity.
pub fn text(content: &str, face: Face, size: f32, color: Color) -> impl Scene + use<> {
    let content = content.to_owned();
    bsn! {
        Text(content)
        font(face, size)
        TextColor(color)
    }
}

/// A small, spaced-out, uppercase caption for labeling sections and fields.
pub fn caption(content: &str) -> impl Scene + use<> {
    let content = content.to_uppercase();
    bsn! {
        text(&content, Face::Strong, 12.0, palette::TEXT_DIM)
        template_value(LetterSpacing::Px(1.6))
    }
}

/// A node filling its parent, centering its children in a column.
pub fn fill() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
    }
}

/// A full-window screen over the menu backdrop gradient.
pub fn backdrop() -> impl Scene {
    let gradient = BackgroundGradient::from(LinearGradient::to_bottom(vec![
        palette::BACKDROP_TOP.into(),
        palette::BACKDROP_BOTTOM.into(),
    ]));
    bsn! {
        fill()
        template_value(gradient)
    }
}

/// A full-window screen dimming the game behind it.
pub fn scrim() -> impl Scene {
    bsn! {
        fill()
        BackgroundColor(palette::SCRIM)
    }
}

/// A bordered panel with an accent strip across its top edge.
pub fn panel(accent: Color) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(24)),
            row_gap: px(12),
            border: UiRect { left: px(2), right: px(2), bottom: px(2), top: px(4) },
        }
        BackgroundColor(palette::PANEL)
        template_value(BorderColor {
            top: accent,
            ..BorderColor::all(palette::PANEL_BORDER)
        })
    }
}

//! The shared look of the client UI: one palette, one variable font, and the text scenes every
//! screen is built from.
//!
//! The look is calm and neutral: greys only, rounded corners, translucent dark panels over the
//! game, hairline separators, and a near-white accent for whatever is interactive. All text uses
//! Recursive, whose variable axes give a heavy display face, a regular sans, and a monospace face (for numbers,
//! keys, and the debug overlay) from one asset.

use bevy::{
    prelude::*,
    text::{FontSourceTemplate, FontVariationTag, FontVariations, FontWeight, LetterSpacing},
};

pub const FONT: &str = "client/font/Recursive_variable.ttf";

/// Keeps [`FONT`] loaded for the app's lifetime. Scenes load it by path, so without a strong
/// handle it unloaded whenever no text was on screen (in game), and the next screen (the pause
/// menu) laid out without text until the font loaded again, jumping a frame later.
#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

impl FromWorld for UiFont {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<AssetServer>().load(FONT))
    }
}

pub mod palette {
    use bevy::color::Color;

    // surfaces: pure greys, with white at low alpha for everything raised above a panel
    pub const BACKDROP_TOP: Color = Color::srgb(0.075, 0.075, 0.075);
    pub const BACKDROP_BOTTOM: Color = Color::srgb(0.015, 0.015, 0.015);
    pub const SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
    pub const PANEL: Color = Color::srgba(0.04, 0.04, 0.04, 0.88);
    pub const PANEL_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.09);
    pub const SURFACE: Color = Color::srgba(1.0, 1.0, 1.0, 0.065);
    pub const SURFACE_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
    pub const SURFACE_PRESSED: Color = Color::srgba(1.0, 1.0, 1.0, 0.18);
    pub const FIELD: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);
    pub const BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.14);
    pub const BORDER_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.32);
    pub const SEPARATOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.07);
    /// The unfilled part of a slider or switch.
    pub const TRACK: Color = Color::srgba(1.0, 1.0, 1.0, 0.16);

    // text
    pub const TEXT: Color = Color::srgb(0.94, 0.94, 0.94);
    pub const TEXT_DIM: Color = Color::srgb(0.68, 0.68, 0.68);
    pub const TEXT_FAINT: Color = Color::srgb(0.46, 0.46, 0.46);
    /// Text on an accent fill.
    pub const TEXT_ON_ACCENT: Color = Color::srgb(0.07, 0.07, 0.07);

    // signal: the single accent is near-white, so the interface stays neutral
    pub const ACCENT: Color = Color::srgb(0.93, 0.93, 0.93);
    pub const ACCENT_HOVER: Color = Color::srgb(1.0, 1.0, 1.0);
    pub const ACCENT_DEEP: Color = Color::srgb(0.32, 0.32, 0.32);
    pub const DANGER: Color = Color::srgb(0.93, 0.45, 0.43);
    pub const DANGER_DEEP: Color = Color::srgba(0.93, 0.35, 0.33, 0.28);

    // debug overlay, in the spirit of the colored F3 mods
    pub const OVERLAY_LINE: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
    pub const LABEL: Color = Color::srgb(1.0, 0.8, 0.32);
    pub const HEADING: Color = Color::srgb(0.96, 0.96, 0.96);
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
        @font(face, size)
        TextColor(color)
    }
}

/// A small, spaced-out, uppercase caption for labeling sections and fields.
pub fn caption(content: &str) -> impl Scene + use<> {
    let content = content.to_uppercase();
    bsn! {
        @text(&content, Face::Strong, 12.0, palette::TEXT_DIM)
        LetterSpacing::Px(1.6)
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
        @fill()
        gradient
    }
}

/// A full-window screen dimming the game behind it.
pub fn scrim() -> impl Scene {
    bsn! {
        @fill()
        BackgroundColor(palette::SCRIM)
    }
}

/// A rounded, translucent panel with a hairline border.
pub fn panel() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(24)),
            row_gap: px(12),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(16)),
        }
        BackgroundColor(palette::PANEL)
        BorderColor::all(palette::PANEL_BORDER)
    }
}

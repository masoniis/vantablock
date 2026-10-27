use crate::ui::theme::{Face, font, palette};
use bevy::{
    input_focus::{InputFocus, tab_navigation::TabIndex},
    picking::hover::Hovered,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::SelectAllOnFocus,
};

/// Marks a styled single-line text field for [`style_text_fields`].
#[derive(Component, Default, Clone)]
pub struct TextField;

/// A single-line monospace text field holding `initial`. Read it through its `EditableText`.
pub fn text_field(initial: &str, max_characters: usize) -> impl Scene + use<> {
    let mut editable = EditableText::new(initial);
    editable.max_characters = Some(max_characters);
    editable.allow_newlines = false;
    bsn! {
        TextField
        template_value(editable)
        template_value(TextLayout::no_wrap())
        SelectAllOnFocus
        Hovered
        TabIndex(0)
        Node {
            // as tall as a button, so the two line up in a row; editable text lays out from the
            // top of its node, so the padding centers it
            height: px(40),
            padding: UiRect::axes(px(12), px(9)),
            border: px(1),
            border_radius: BorderRadius::all(px(10)),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
        }
        font(Face::Mono, 16.0)
        TextColor(palette::TEXT)
        TextCursorStyle {
            color: palette::ACCENT,
            selection_color: {palette::ACCENT.with_alpha(0.3)},
            unfocused_selection_color: Color::NONE,
        }
        BackgroundColor(palette::FIELD)
        BorderColor::all(palette::BORDER)
    }
}

/// Outlines the focused field in the accent and brightens hovered ones.
pub(super) fn style_text_fields(
    focus: Res<InputFocus>,
    mut fields: Query<(Entity, &Hovered, &mut BorderColor), With<TextField>>,
) {
    for (entity, hovered, mut border) in &mut fields {
        let edge = if focus.get() == Some(entity) {
            palette::ACCENT
        } else if hovered.get() {
            palette::BORDER_HOVER
        } else {
            palette::BORDER
        };
        border.set_if_neq(BorderColor::all(edge));
    }
}

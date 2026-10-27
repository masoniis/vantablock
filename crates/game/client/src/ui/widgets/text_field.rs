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
            // editable text lays out from the top of its node, so padding centers it
            padding: UiRect::axes(px(12), px(10)),
            border: px(2),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
        }
        font(Face::Mono, 17.0)
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

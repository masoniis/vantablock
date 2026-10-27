use crate::ui::theme::{Face, palette, text};
use bevy::{
    input_focus::{InputFocus, InputFocusVisible, tab_navigation::TabIndex},
    picking::hover::Hovered,
    prelude::*,
    text::LetterSpacing,
    ui::{InteractionDisabled, Pressed},
    ui_widgets,
};

/// How prominent a button is. Also marks the entity for [`style_buttons`].
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonTone {
    #[default]
    Normal,
    /// The screen's main action, outlined in the accent.
    Primary,
    /// A destructive or exiting action, which warms to red on hover.
    Danger,
}

/// A blocky button with a caption. Attach behavior with `on(|_: On<Activate>| ...)`.
pub fn button(label: &str, tone: ButtonTone) -> impl Scene + use<> {
    let label = label.to_uppercase();
    bsn! {
        ui_widgets::Button
        template_value(tone)
        Hovered
        TabIndex(0)
        Node {
            height: px(44),
            padding: UiRect::horizontal(px(18)),
            border: px(2),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor(palette::SURFACE)
        BorderColor::all(palette::BORDER)
        Children [(
            text(&label, Face::Strong, 16.0, palette::TEXT)
            template_value(LetterSpacing::Px(1.4))
        )]
    }
}

/// Restyles every button from its tone, hover, press, and keyboard focus.
pub(super) fn style_buttons(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    mut buttons: Query<(
        Entity,
        &ButtonTone,
        &Hovered,
        Has<Pressed>,
        Has<InteractionDisabled>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (entity, tone, hovered, pressed, disabled, mut background, mut border) in &mut buttons {
        let (signal, deep) = match tone {
            ButtonTone::Danger => (palette::DANGER, palette::DANGER_DEEP),
            _ => (palette::ACCENT, palette::ACCENT_DEEP),
        };
        let focused = focus_visible.0 && focus.get() == Some(entity);
        let hot = hovered.get() || focused;

        let (fill, edge) = if disabled {
            (palette::SURFACE, palette::PANEL_BORDER)
        } else if pressed {
            (deep, signal)
        } else if hot {
            (palette::SURFACE_HOVER, signal)
        } else if *tone == ButtonTone::Primary {
            (palette::SURFACE, palette::ACCENT_DEEP.lighter(0.12))
        } else {
            (palette::SURFACE, palette::BORDER)
        };
        background.set_if_neq(BackgroundColor(fill));
        border.set_if_neq(BorderColor::all(edge));
    }
}

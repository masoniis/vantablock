use crate::ui::theme::{Face, palette, text};
use bevy::{
    input_focus::{InputFocus, InputFocusVisible, tab_navigation::TabIndex},
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets,
};

/// How prominent a button is. Also marks the entity for [`style_buttons`].
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonTone {
    #[default]
    Normal,
    /// The screen's main action, filled with the accent.
    Primary,
    /// A destructive or exiting action, which warms to red on hover.
    Danger,
    /// A low-key action: no fill until hovered.
    Quiet,
}

/// A rounded button with a caption. Attach behavior with `on(|_: On<Activate>| ...)`.
pub fn button(label: &str, tone: ButtonTone) -> impl Scene + use<> {
    let label = label.to_owned();
    bsn! {
        ui_widgets::Button
        tone
        Hovered
        TabIndex(0)
        Node {
            height: px(40),
            padding: UiRect::horizontal(px(18)),
            // a transparent border the keyboard focus ring fills in
            border: px(1),
            border_radius: BorderRadius::all(px(10)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor(palette::SURFACE)
        BorderColor::all(Color::NONE)
        Children [ @text(&label, Face::Strong, 15.0, palette::TEXT) ]
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
        &Children,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut labels: Query<&mut TextColor>,
) {
    for (entity, tone, hovered, pressed, disabled, children, mut background, mut border) in
        &mut buttons
    {
        let focused = focus_visible.0 && focus.get() == Some(entity);
        let hot = hovered.get() || focused;
        let danger = *tone == ButtonTone::Danger;

        let (fill, label) = match (*tone, disabled, pressed, hot) {
            (_, true, ..) => (
                if *tone == ButtonTone::Primary {
                    palette::SURFACE
                } else {
                    Color::NONE
                },
                palette::TEXT_FAINT,
            ),
            (ButtonTone::Primary, _, true, _) => {
                (palette::ACCENT.darker(0.12), palette::TEXT_ON_ACCENT)
            }
            (ButtonTone::Primary, _, _, true) => (palette::ACCENT_HOVER, palette::TEXT_ON_ACCENT),
            (ButtonTone::Primary, ..) => (palette::ACCENT, palette::TEXT_ON_ACCENT),
            (_, _, true, _) if danger => (palette::DANGER_DEEP, palette::DANGER),
            (_, _, true, _) => (palette::SURFACE_PRESSED, palette::TEXT),
            (_, _, _, true) if danger => (palette::DANGER_DEEP.with_alpha(0.16), palette::DANGER),
            (_, _, _, true) => (palette::SURFACE_HOVER, palette::TEXT),
            (ButtonTone::Quiet, ..) => (Color::NONE, palette::TEXT_DIM),
            _ => (palette::SURFACE, palette::TEXT),
        };
        let edge = if focused {
            palette::BORDER_HOVER
        } else {
            Color::NONE
        };
        background.set_if_neq(BackgroundColor(fill));
        border.set_if_neq(BorderColor::all(edge));
        for &child in children {
            if let Ok(mut color) = labels.get_mut(child) {
                color.set_if_neq(TextColor(label));
            }
        }
    }
}

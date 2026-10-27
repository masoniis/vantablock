use crate::{
    input::{
        input_maps::{client_keys, player_keys},
        local_actions::ClientAction,
    },
    lifecycle::state::{ClientLifecycleState, InGameState},
    ui::{
        theme::{Face, caption, palette, panel, scrim, text},
        widgets::{ButtonTone, button, keycap, keycaps},
    },
};
use bevy::{
    input_focus::tab_navigation::TabGroup, prelude::*, text::LetterSpacing, ui_widgets::Activate,
};
use common::player::PlayerAction;

#[derive(Component, Default, Clone)]
pub struct PauseMenuRoot;

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(InGameState::Paused), spawn_pause_menu);
    }
}

fn spawn_pause_menu(mut commands: Commands) {
    info!("Spawning Pause Menu UI...");
    commands
        .spawn_scene(pause_menu())
        .insert(DespawnOnExit(InGameState::Paused));
}

fn pause_menu() -> impl Scene {
    bsn! {
        PauseMenuRoot
        scrim()
        TabGroup
        Children [(
            Node { column_gap: px(16), align_items: AlignItems::FlexStart }
            Children [
                (
                    panel(palette::ACCENT)
                    Node { width: px(260) }
                    Children [
                        (
                            text("PAUSED", Face::Display, 30.0, palette::TEXT)
                            template_value(LetterSpacing::Px(3.0))
                            Node { margin: UiRect::bottom(px(8)) }
                        ),
                        (
                            button("Resume", ButtonTone::Primary)
                            on(|_: On<Activate>, mut next: ResMut<NextState<InGameState>>| {
                                next.set(InGameState::Playing);
                            })
                        ),
                        (
                            button("Disconnect", ButtonTone::Danger)
                            on(|_: On<Activate>, mut next: ResMut<NextState<ClientLifecycleState>>| {
                                // leaving the game disconnects and cleans up the session
                                next.set(ClientLifecycleState::MainMenu);
                            })
                        ),
                        (
                            Node { column_gap: px(8), align_items: AlignItems::Center, margin: UiRect::top(px(8)) }
                            Children [
                                keycap("Esc"),
                                text("to resume", Face::Sans, 13.0, palette::TEXT_DIM),
                            ]
                        ),
                    ]
                ),
                (
                    panel(palette::PANEL_BORDER)
                    Node { width: px(400), row_gap: px(6) }
                    Children [
                        (
                            text("CONTROLS", Face::Display, 18.0, palette::TEXT)
                            template_value(LetterSpacing::Px(2.0))
                        ),
                        {controls()},
                    ]
                ),
            ]
        )]
    }
}

/// The controls reference: one section per kind of action, each row read from the bindings.
fn controls() -> Vec<Box<dyn Scene>> {
    use ClientAction as C;
    use PlayerAction as P;
    let player = |actions: &[P]| actions.iter().map(|&a| player_keys(a)).collect::<Vec<_>>();
    let client = |actions: &[C]| actions.iter().map(|&a| client_keys(a)).collect::<Vec<_>>();

    vec![
        section("Movement"),
        row(
            "Move",
            bindings(&player(&[
                P::MoveForward,
                P::MoveLeft,
                P::MoveBackward,
                P::MoveRight,
            ])),
        ),
        row("Sprint", bindings(&player(&[P::MoveFaster]))),
        row("Look", Box::new(keycap("Mouse"))),
        row("Zoom", Box::new(keycap("Scroll"))),
        section("Blocks"),
        row("Break", Box::new(keycap("Left click"))),
        row("Place", Box::new(keycap("Right click"))),
        section("Interface"),
        row("Pause menu", bindings(&client(&[C::TogglePause]))),
        row("Debug overlay", bindings(&client(&[C::ToggleDebugMenu]))),
        row("Diagnostics", bindings(&client(&[C::ToggleDiagnostics]))),
        section("World clock"),
        row(
            "Time back / forward",
            bindings(&player(&[P::JumpGameTimeBackward, P::JumpGameTimeForward])),
        ),
        row("Pause time", bindings(&player(&[P::PauseGameTime]))),
        section("Debug views"),
        row(
            "Wireframe",
            bindings(&client(&[C::ToggleOpaqueWireframeMode])),
        ),
        row("Chunk borders", bindings(&client(&[C::ToggleChunkBorders]))),
    ]
}

fn section(title: &str) -> Box<dyn Scene> {
    Box::new(bsn! {
        caption(title)
        Node { margin: UiRect { top: px(10), bottom: px(2) } }
    })
}

fn row(label: &str, keys: Box<dyn Scene>) -> Box<dyn Scene> {
    Box::new(bsn! {
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            min_height: px(26),
        }
        Children [
            text(label, Face::Sans, 14.0, palette::TEXT),
            ({keys}),
        ]
    })
}

/// Keycaps for a row's actions: each action's keys side by side, an action's alternative keys
/// separated by a slash.
fn bindings(actions: &[&[KeyCode]]) -> Box<dyn Scene> {
    let mut parts: Vec<Box<dyn Scene>> = Vec::new();
    for keys in actions {
        for (i, &key) in keys.iter().enumerate() {
            if i > 0 {
                parts.push(Box::new(text("/", Face::Mono, 12.0, palette::TEXT_FAINT)));
            }
            parts.push(Box::new(keycaps(&[key])));
        }
    }
    Box::new(bsn! {
        Node { column_gap: px(4), align_items: AlignItems::Center }
        Children [ {parts} ]
    })
}

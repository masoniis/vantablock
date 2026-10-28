use crate::{
    debug::showcase::{ApplyShowcase, SHOWCASES},
    input::{
        input_maps::{client_keys, player_keys},
        local_actions::ClientAction,
    },
    lifecycle::state::{ClientLifecycleState, InGameState},
    network::connection::SessionWorld,
    ui::{
        theme::{Face, caption, palette, panel, text},
        widgets::{ButtonTone, button, keycap, keycaps},
    },
};
use bevy::{input_focus::tab_navigation::TabGroup, prelude::*, ui_widgets::Activate};
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
    show_page(&mut commands, None, pause_menu());
}

/// Replaces the open pause page (if any) with `page`, which closes when the game resumes.
fn show_page(commands: &mut Commands, open: Option<Entity>, page: impl Scene) {
    if let Some(open) = open {
        commands.entity(open).despawn();
    }
    commands
        .spawn_scene(page)
        .insert(DespawnOnExit(InGameState::Paused));
}

fn pause_menu() -> impl Scene {
    // docked left, so the game stays visible on the right
    bsn! {
        PauseMenuRoot
        docked()
        TabGroup
        Children [(
            Node { column_gap: px(12), align_items: AlignItems::FlexStart }
            Children [
                (
                    panel()
                    Node { width: px(280), padding: UiRect::all(px(28)), row_gap: px(10) }
                    Children [
                        (
                            text("Paused", Face::Strong, 24.0, palette::TEXT)
                            Node { margin: UiRect::bottom(px(10)) }
                        ),
                        (
                            button("Resume", ButtonTone::Primary)
                            on(|_: On<Activate>, mut next: ResMut<NextState<InGameState>>| {
                                next.set(InGameState::Playing);
                            })
                        ),
                        (
                            button("Showcase", ButtonTone::Normal)
                            on(|_: On<Activate>, mut commands: Commands, page: Single<Entity, With<PauseMenuRoot>>, world: Option<Res<SessionWorld>>| {
                                show_page(&mut commands, Some(*page), showcase_page(world.as_deref().copied()));
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
                            Node {
                                column_gap: px(8),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                margin: UiRect::top(px(8)),
                            }
                            Children [
                                keycap("Esc"),
                                text("to resume", Face::Sans, 13.0, palette::TEXT_DIM),
                            ]
                        ),
                    ]
                ),
                (
                    panel()
                    Node { width: px(400), row_gap: px(6) }
                    Children [
                        text("Controls", Face::Strong, 18.0, palette::TEXT),
                        {controls()},
                    ]
                ),
            ]
        )]
    }
}

/// A list of the showcase scenes for this world's terrain (all of them when joining a server,
/// whose terrain the client doesn't know). Picking one jumps there and resumes the game.
fn showcase_page(world: Option<SessionWorld>) -> impl Scene {
    let mut scenes: Vec<Box<dyn Scene>> = SHOWCASES
        .iter()
        .enumerate()
        .filter(|(_, showcase)| world.is_none_or(|world| world.0.shaper == showcase.shaper))
        .map(|(index, showcase)| -> Box<dyn Scene> {
            Box::new(bsn! {
                button(showcase.name, ButtonTone::Normal)
                on(move |_: On<Activate>, mut commands: Commands, mut next: ResMut<NextState<InGameState>>| {
                    commands.trigger(ApplyShowcase(index));
                    next.set(InGameState::Playing);
                })
            })
        })
        .collect();
    if scenes.is_empty() {
        scenes.push(Box::new(text(
            "No scenes for this terrain",
            Face::Sans,
            14.0,
            palette::TEXT_DIM,
        )));
    }

    bsn! {
        PauseMenuRoot
        docked()
        TabGroup
        Children [(
            panel()
            Node { width: px(280), padding: UiRect::all(px(28)), row_gap: px(10) }
            Children [
                (
                    text("Showcase", Face::Strong, 24.0, palette::TEXT)
                    Node { margin: UiRect::bottom(px(10)) }
                ),
                {scenes},
                (
                    button("Back", ButtonTone::Quiet)
                    Node { margin: UiRect::top(px(8)) }
                    on(|_: On<Activate>, mut commands: Commands, page: Single<Entity, With<PauseMenuRoot>>| {
                        show_page(&mut commands, Some(*page), pause_menu());
                    })
                ),
            ]
        )]
    }
}

/// A full-window layer that docks its content to the left edge, vertically centered.
fn docked() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexStart,
            padding: UiRect::left(px(16)),
        }
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

use crate::{
    lifecycle::state::ClientLifecycleState,
    network::connection::{ConnectType, InitiateConnection},
    ui::{
        systems::spawn_menu_camera_system,
        theme::{Face, backdrop, caption, palette, text},
        widgets::{ButtonTone, button, text_field},
    },
};
use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, tab_navigation::TabGroup},
    prelude::*,
    text::{EditableText, LetterSpacing},
    ui_widgets::Activate,
};
use common::network::DEFAULT_LOCAL_SERVER_ADDR;

#[derive(Component, Default, Clone)]
pub struct MainMenuUiRoot;

/// The server address field, read when connecting.
#[derive(Component, Default, Clone)]
pub struct ServerAddrInput;

pub struct MainMenuUiPlugin;

impl Plugin for MainMenuUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ClientLifecycleState::MainMenu),
            (spawn_menu_camera_system, spawn_main_menu),
        );
    }
}

fn spawn_main_menu(mut commands: Commands) {
    info!("Spawning Main Menu UI...");
    commands
        .spawn_scene(main_menu())
        .insert(DespawnOnExit(ClientLifecycleState::MainMenu));
}

fn main_menu() -> impl Scene {
    bsn! {
        MainMenuUiRoot
        backdrop()
        TabGroup
        Children [
            (
                Node { width: px(380), flex_direction: FlexDirection::Column, row_gap: px(10) }
                Children [
                    title(),
                    (
                        button("Singleplayer", ButtonTone::Primary)
                        Node { margin: UiRect::top(px(28)) }
                        // singleplayer always joins its own background server
                        on(|_: On<Activate>, mut commands: Commands| {
                            commands.trigger(InitiateConnection {
                                connect_type: ConnectType::Singleplayer,
                                server_addr: DEFAULT_LOCAL_SERVER_ADDR.to_owned(),
                            });
                        })
                    ),
                    (caption("Server address") Node { margin: UiRect::top(px(18)) }),
                    (
                        Node { column_gap: px(8) }
                        Children [
                            (
                                text_field(DEFAULT_LOCAL_SERVER_ADDR, 64)
                                ServerAddrInput
                                Node { flex_grow: 1.0 }
                                on(join_on_enter)
                            ),
                            (
                                button("Join", ButtonTone::Normal)
                                on(|_: On<Activate>, addr: Single<&EditableText, With<ServerAddrInput>>, mut commands: Commands| {
                                    join(&mut commands, &addr);
                                })
                            ),
                        ]
                    ),
                ]
            ),
            footer(),
        ]
    }
}

/// The wordmark: a heavy title over an accent rule.
fn title() -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center }
        Children [
            (
                text("VANTABLOCK", Face::Display, 60.0, palette::TEXT)
                template_value(LetterSpacing::Px(5.0))
                TextShadow { offset: Vec2 { x: 0.0, y: 4.0 }, color: {palette::ACCENT_DEEP} }
            ),
            (
                Node { width: px(56), height: px(4), margin: UiRect::top(px(10)) }
                BackgroundColor(palette::ACCENT)
            ),
        ]
    }
}

fn footer() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(16),
            right: px(16),
            bottom: px(12),
        }
        Children [
            text(concat!("v", env!("CARGO_PKG_VERSION")), Face::Mono, 12.0, palette::TEXT_FAINT),
        ]
    }
}

/// Pressing Enter in the address field joins that server.
fn join_on_enter(
    key: On<FocusedInput<KeyboardInput>>,
    addr: Single<&EditableText, With<ServerAddrInput>>,
    mut commands: Commands,
) {
    if key.input.key_code == KeyCode::Enter && key.input.state == ButtonState::Pressed {
        join(&mut commands, &addr);
    }
}

fn join(commands: &mut Commands, addr: &EditableText) {
    commands.trigger(InitiateConnection {
        connect_type: ConnectType::Multiplayer,
        server_addr: addr.value().to_string().trim().to_owned(),
    });
}

use crate::{
    lifecycle::state::ClientLifecycleState,
    network::connection::{ConnectType, InitiateConnection},
    ui::systems::spawn_menu_camera_system,
};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use shared::network::DEFAULT_LOCAL_SERVER_ADDR;

#[derive(Component)]
pub struct MainMenuUiRoot;

#[derive(Component)]
pub enum MainMenuButtonAction {
    Singleplayer,
    Multiplayer,
}

#[derive(Component)]
pub struct ServerAddrInput;

pub struct MainMenuUiPlugin;

impl Plugin for MainMenuUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ClientLifecycleState::MainMenu),
            (spawn_menu_camera_system, spawn_main_menu),
        )
        .add_systems(
            Update,
            (
                main_menu_button_interaction_system,
                main_menu_text_input_system,
            )
                .run_if(in_state(ClientLifecycleState::MainMenu)),
        );
    }
}

pub fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("Spawning Main Menu UI...");

    let font = asset_server.load("client/font/Recursive_variable.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            MainMenuUiRoot,
            DespawnOnExit(ClientLifecycleState::MainMenu),
        ))
        .with_children(|parent| {
            // title
            parent.spawn((
                Text::new("VANTABLOCK"),
                TextFont {
                    font: font.clone(),
                    font_size: 72.0,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..Default::default()
                },
            ));

            // singleplayer button
            spawn_button(
                parent,
                font.clone(),
                "SINGLEPLAYER",
                MainMenuButtonAction::Singleplayer,
            );

            // spacer
            parent.spawn(Node {
                height: Val::Px(20.0),
                ..default()
            });

            // multiplayer button
            spawn_button(
                parent,
                font.clone(),
                "MULTIPLAYER",
                MainMenuButtonAction::Multiplayer,
            );

            // server address input label
            parent.spawn((
                Text::new("Server Address:"),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..Default::default()
                },
            ));

            // input box
            parent
                .spawn((
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(40.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(5.0)),
                        ..Default::default()
                    },
                    BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
                    BackgroundColor(Color::BLACK),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(DEFAULT_LOCAL_SERVER_ADDR),
                        TextFont {
                            font: font.clone(),
                            font_size: 24.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        ServerAddrInput,
                    ));
                });
        });
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    action: MainMenuButtonAction,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(300.0),
                height: Val::Px(65.0),
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BorderColor::all(Color::WHITE),
            BackgroundColor(Color::LinearRgba(LinearRgba::new(0.1, 0.1, 0.1, 0.9))),
            action,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(label),
                TextFont {
                    font,
                    font_size: 32.0,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn main_menu_button_interaction_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &MainMenuButtonAction,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut commands: Commands,
    server_addr_query: Query<&Text, With<ServerAddrInput>>,
) {
    for (interaction, mut color, mut border_color, action) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::LinearRgba(LinearRgba::new(0.3, 0.3, 0.3, 1.0)));
                *border_color = BorderColor::all(Color::WHITE);

                let server_addr = server_addr_query.single().unwrap().0.clone();

                match action {
                    MainMenuButtonAction::Singleplayer => {
                        commands.trigger(InitiateConnection {
                            connect_type: ConnectType::Singleplayer,
                            server_addr,
                        });
                    }
                    MainMenuButtonAction::Multiplayer => {
                        commands.trigger(InitiateConnection {
                            connect_type: ConnectType::Multiplayer,
                            server_addr,
                        });
                    }
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::LinearRgba(LinearRgba::new(0.2, 0.2, 0.2, 0.9)));
                *border_color = BorderColor::all(Color::WHITE);
            }
            Interaction::None => {
                *color = BackgroundColor(Color::LinearRgba(LinearRgba::new(0.1, 0.1, 0.1, 0.9)));
                *border_color = BorderColor::all(Color::WHITE);
            }
        }
    }
}

pub fn main_menu_text_input_system(
    mut char_evr: MessageReader<KeyboardInput>,
    mut query: Query<&mut Text, With<ServerAddrInput>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };

    for event in char_evr.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Character(c) => {
                text.0.push_str(c.as_str());
            }
            Key::Backspace => {
                text.0.pop();
            }
            _ => {}
        }
    }
}

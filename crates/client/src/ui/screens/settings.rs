use crate::lifecycle::state::{ClientLifecycleState, InGameState};
use bevy::prelude::*;
use lightyear::prelude::client::{Disconnect, NetcodeClient};

#[derive(Component)]
pub struct SettingsUiRoot;

#[derive(Component)]
pub enum SettingsButtonAction {
    Resume,
    Disconnect,
}

pub struct SettingsUiPlugin;

impl Plugin for SettingsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(InGameState::Paused), spawn_settings_ui)
            .add_systems(
                Update,
                settings_button_interaction_system.run_if(in_state(InGameState::Paused)),
            );
    }
}

pub fn spawn_settings_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("Spawning Settings UI...");

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
            BackgroundColor(Color::LinearRgba(LinearRgba::new(0.0, 0.0, 0.0, 0.5))),
            SettingsUiRoot,
            DespawnOnExit(InGameState::Paused),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font: font.clone(),
                    font_size: 64.0,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..Default::default()
                },
            ));

            spawn_button(parent, font.clone(), "RESUME", SettingsButtonAction::Resume);

            parent.spawn(Node {
                height: Val::Px(20.0),
                ..default()
            });

            spawn_button(
                parent,
                font.clone(),
                "DISCONNECT",
                SettingsButtonAction::Disconnect,
            );
        });
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    action: SettingsButtonAction,
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

pub fn settings_button_interaction_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &SettingsButtonAction,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_in_game_state: ResMut<NextState<InGameState>>,
    mut next_client_state: ResMut<NextState<ClientLifecycleState>>,
    mut commands: Commands,
    client_query: Query<Entity, With<NetcodeClient>>,
) {
    for (interaction, mut color, mut border_color, action) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::LinearRgba(LinearRgba::new(0.3, 0.3, 0.3, 1.0)));
                *border_color = BorderColor::all(Color::WHITE);

                match action {
                    SettingsButtonAction::Resume => {
                        next_in_game_state.set(InGameState::Playing);
                    }
                    SettingsButtonAction::Disconnect => {
                        // trigger lightyear disconnect
                        for entity in client_query.iter() {
                            commands.trigger(Disconnect { entity });
                        }

                        next_client_state.set(ClientLifecycleState::MainMenu);
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

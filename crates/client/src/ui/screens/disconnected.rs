use crate::lifecycle::state::ClientState;
use crate::network::connection::NetworkErrorEvent;
use bevy::prelude::*;

#[derive(Component)]
pub struct DisconnectedUiRoot;

#[derive(Component)]
pub struct RetryButton;

pub fn spawn_disconnected_ui(
    trigger: On<NetworkErrorEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let event = trigger.event();
    let reason_text = event.reason.clone();

    info!("Spawning Disconnected UI (Reason: {})...", reason_text);

    let font = asset_server.load("client/font/Recursive_variable.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            DisconnectedUiRoot,
            DespawnOnExit(ClientState::Error),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(500.0),
                        height: Val::Px(250.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::LinearRgba(LinearRgba::new(0.1, 0.02, 0.02, 0.9))),
                    Outline::new(Val::Px(2.0), Val::Px(0.0), Color::srgb(1.0, 0.3, 0.3)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("DISCONNECTED"),
                        TextFont {
                            font: font.clone(),
                            font_size: 48.0,
                            ..Default::default()
                        },
                        TextColor(Color::srgb(1.0, 0.2, 0.2)),
                    ));

                    parent.spawn((
                        Text::new(reason_text.clone()),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            margin: UiRect::vertical(Val::Px(20.0)),
                            ..Default::default()
                        },
                    ));

                    // retry button (back to main menu)
                    parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(200.0),
                                height: Val::Px(50.0),
                                border: UiRect::all(Val::Px(2.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..Default::default()
                            },
                            BorderColor::all(Color::WHITE),
                            BackgroundColor(Color::LinearRgba(LinearRgba::new(0.2, 0.2, 0.2, 0.9))),
                            RetryButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("BACK TO MENU"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..Default::default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}

pub fn disconnected_ui_button_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, With<RetryButton>),
    >,
    mut next_client_state: ResMut<NextState<ClientState>>,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                next_client_state.set(ClientState::MainMenu);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::LinearRgba(LinearRgba::new(0.3, 0.3, 0.3, 0.9)));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::LinearRgba(LinearRgba::new(0.2, 0.2, 0.2, 0.9)));
            }
        }
    }
}

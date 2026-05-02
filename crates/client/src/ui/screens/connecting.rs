use crate::lifecycle::state::enums::InGameState;
use bevy::prelude::*;

#[derive(Component)]
pub struct ConnectingUiRoot;

pub fn spawn_connecting_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("Spawning Connecting UI...");

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
            ConnectingUiRoot,
            DespawnOnExit(InGameState::Connecting),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(150.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::LinearRgba(LinearRgba::new(0.05, 0.05, 0.05, 0.8))),
                    Outline::new(Val::Px(2.0), Val::Px(0.0), Color::WHITE),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Connecting..."),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    parent.spawn((
                        Text::new("Establishing connection..."),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..Default::default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        Node {
                            margin: UiRect::top(Val::Px(10.0)),
                            ..Default::default()
                        },
                    ));
                });
        });
}

use crate::lifecycle::state::ClientGameState;
use bevy::prelude::*;

#[derive(Component)]
pub struct MainMenuUiRoot;

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
            DespawnOnExit(ClientGameState::MainMenu),
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

            // play button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(250.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BorderColor::all(Color::WHITE),
                    BackgroundColor(Color::LinearRgba(LinearRgba::new(0.1, 0.1, 0.1, 0.9))),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("PLAY"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn main_menu_button_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<ClientGameState>>,
) {
    for (interaction, mut color, mut border_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::LinearRgba(LinearRgba::new(0.3, 0.3, 0.3, 1.0)));
                *border_color = BorderColor::all(Color::WHITE);
                // transition to playing state
                next_state.set(ClientGameState::Playing);
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

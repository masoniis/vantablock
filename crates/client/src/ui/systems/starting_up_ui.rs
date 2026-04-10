use bevy::prelude::*;
use shared::lifecycle::state::SimulationState;

#[derive(Component)]
pub struct StartingUpUiRoot;

pub fn spawn_starting_up_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("Spawning StartingUp UI...");

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
            StartingUpUiRoot,
            DespawnOnExit(SimulationState::Loading),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(100.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::LinearRgba(LinearRgba::new(0.05, 0.05, 0.05, 0.8))),
                    Outline::new(Val::Px(2.0), Val::Px(0.0), Color::WHITE),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("StartingUp..."),
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

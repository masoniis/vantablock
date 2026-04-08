use bevy::prelude::*;

// INFO: -----------------------
//         ui components
// -----------------------------

#[derive(Component)]
pub struct VantablockUiRoot;

// INFO: -----------------------------
//         ui spawning systems
// -----------------------------------

pub fn spawn_ui_system(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    info!("Spawning Vantablock UI...");

    let font = asset_server.load("client/font/Recursive_variable.ttf");

    // root node
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
            VantablockUiRoot,
        ))
        .with_children(|parent| {
            // centered box with standard background color
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(200.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::LinearRgba(LinearRgba::new(0.1, 0.1, 0.1, 0.5))),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Vantablock"),
                        TextFont {
                            font: font.clone(),
                            font_size: 40.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // hotbar-like container at the bottom
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(20.0),
                    width: Val::Percent(50.0),
                    height: Val::Px(60.0),
                    display: Display::Flex,
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                BackgroundColor(Color::LinearRgba(LinearRgba::new(0.0, 0.0, 0.0, 0.3))),
            ));
        });
}

pub fn despawn_ui_system(mut commands: Commands, query: Query<Entity, With<VantablockUiRoot>>) {
    info!("Despawning Vantablock UI...");
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

use bevy::prelude::*;

// INFO: -----------------------
//         ui components
// -----------------------------

#[derive(Component)]
pub struct VantablockUiRoot;

// INFO: -----------------------------
//         ui spawning systems
// -----------------------------------

pub fn spawn_ui_root(mut commands: Commands, _asset_server: ResMut<AssetServer>) {
    info!("Spawning root UI...");

    // root node
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        VantablockUiRoot,
    ));
}

pub fn despawn_ui_root(mut commands: Commands, query: Query<Entity, With<VantablockUiRoot>>) {
    info!("Despawning root UI...");
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

use bevy::prelude::*;

/// Marker for the temporary main menu camera.
#[derive(Component)]
pub struct MainMenuCamera;

/// A system that spawns a temporary camera for the lobby/menu.
/// This camera will be despawned once the authoritative player entity is received.
pub fn spawn_menu_camera_system(
    mut commands: Commands,
    query: Query<Entity, With<MainMenuCamera>>,
) {
    if !query.is_empty() {
        return;
    }

    info!("Spawning 2d menu camera.");

    commands.spawn((MainMenuCamera, Camera2d));
}

/// A system that despawns the temporary lobby camera.
pub fn despawn_menu_camera_system(
    mut commands: Commands,
    query: Query<Entity, With<MainMenuCamera>>,
) {
    for entity in query.iter() {
        info!("Despawning 2d menu camera.");
        commands.entity(entity).despawn();
    }
}

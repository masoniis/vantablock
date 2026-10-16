use bevy::prelude::{Component, Reflect, ReflectComponent};

/// A client-only marker indicating that this player entity belongs to the local user.
/// Used for camera tracking and local input gathering. Do NOT network this.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct LocalPlayer;

/// Marker for the local player's camera.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct LocalPlayerCamera;

/// Stores the local player's absolute look orientation (Yaw/Pitch).
/// This is used as a persistent source of truth on the client that is NOT
/// subject to network replication rollback.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct LocalPlayerLook {
    pub yaw: f32,
    pub pitch: f32,
}

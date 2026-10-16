use bevy::prelude::Component;

/// A client-only marker indicating that this player entity belongs to the local user.
/// Used for camera tracking and local input gathering. Do NOT network this.
#[derive(Component)]
pub struct LocalPlayer;

/// Marker for the local player's camera.
#[derive(Component)]
pub struct LocalPlayerCamera;

/// Stores the local player's absolute look orientation (Yaw/Pitch).
/// This is used as a persistent source of truth on the client that is NOT
/// subject to network replication rollback.
#[derive(Component, Default)]
pub struct LocalPlayerLook {
    pub yaw: f32,
    pub pitch: f32,
}

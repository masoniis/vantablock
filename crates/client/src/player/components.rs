use bevy::prelude::Component;

/// A client-only marker indicating that this player entity belongs to the local user.
/// Used for camera tracking and local input gathering. Do NOT network this.
#[derive(Component)]
pub struct LocalPlayer;

/// Marker for the local player's camera.
#[derive(Component)]
pub struct LocalPlayerCamera;

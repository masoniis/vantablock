use bevy::prelude::*;

#[derive(Component)]
pub struct ClientConnection {
    pub client_entity: Entity,
}

/// How often the world clock is broadcast to clients.
#[derive(Resource)]
pub struct ClockSyncTimer(pub Timer);

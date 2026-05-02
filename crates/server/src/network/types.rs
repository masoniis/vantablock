use bevy::prelude::*;

#[derive(Component)]
pub struct ClientConnection {
    pub client_entity: Entity,
}

#[derive(Resource)]
pub struct MessageTimer(pub Timer);

use bevy::ecs::prelude::Component;

/// A marker component to indicate that a chunk's mesh needs to be regenerated.
#[derive(Component)]
pub struct ChunkMeshDirty;

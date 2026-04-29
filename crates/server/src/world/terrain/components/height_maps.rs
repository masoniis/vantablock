use bevy::ecs::prelude::Component;

use shared::world::chunk::CHUNK_AREA;

/// Heightmap of the highest non-transparent block.
///
/// Necessary for optimized lighting calculations.
#[derive(Component, Clone)]
pub struct OceanFloorHeightMapComponent(pub [u16; CHUNK_AREA]);

impl OceanFloorHeightMapComponent {
    /// Creates a new empty surface heightmap.
    pub fn empty() -> Self {
        Self([0; CHUNK_AREA])
    }
}

/// Heightmap of the highest solid block.
///
/// Necessary for spawning the player or decorations.
#[derive(Component, Clone)]
pub struct WorldSurfaceHeightMapComponent(pub [u16; CHUNK_AREA]);

impl WorldSurfaceHeightMapComponent {
    /// Creates a new empty surface heightmap.
    pub fn empty() -> Self {
        Self([0; CHUNK_AREA])
    }
}

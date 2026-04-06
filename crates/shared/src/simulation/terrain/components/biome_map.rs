use crate::prelude::*;
use crate::simulation::biome::biome_registry::BiomeId;
use crate::simulation::chunk::types::{ChunkLod, ChunkVolumeData};
use bevy::ecs::prelude::Component;

#[derive(Component, Clone, Deref, DerefMut)]
pub struct BiomeMapComponent(pub ChunkVolumeData<BiomeId>);

impl BiomeMapComponent {
    pub fn new_empty(lod: ChunkLod) -> Self {
        Self(ChunkVolumeData::new_zeroed(lod))
    }
}

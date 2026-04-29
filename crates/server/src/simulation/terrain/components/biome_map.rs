use bevy::prelude::{Component, Deref, DerefMut};
use shared::world::biome::biome_registry::BiomeId;
use shared::world::chunk::types::{ChunkLod, ChunkVolumeData};

#[derive(Component, Clone, Deref, DerefMut)]
pub struct BiomeMapComponent(pub ChunkVolumeData<BiomeId>);

impl BiomeMapComponent {
    pub fn new_empty(lod: ChunkLod) -> Self {
        Self(ChunkVolumeData::new_zeroed(lod))
    }
}

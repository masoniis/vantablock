use crate::world::terrain::{
    biome::MultiNoiseBiomeGenerator, generators::biome::lib::BiomeGenerator,
};
use bevy::ecs::prelude::Resource;
use std::sync::Arc;

/// A resource holding the active biome chunk generator.
#[derive(Resource, Clone)]
pub struct ActiveBiomeGenerator(pub Arc<dyn BiomeGenerator + Send + Sync>);

impl Default for ActiveBiomeGenerator {
    fn default() -> Self {
        Self(Arc::new(MultiNoiseBiomeGenerator))
    }
}

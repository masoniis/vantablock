use crate::world::terrain::generators::climate::{
    climate_noise_gen::ClimateNoiseGenerator, lib::ClimateGenerator,
};
use bevy::ecs::prelude::Resource;
use std::sync::Arc;

#[derive(Resource, Clone)]
pub struct ActiveClimateGenerator(pub Arc<dyn ClimateGenerator + Send + Sync>);

impl Default for ActiveClimateGenerator {
    fn default() -> Self {
        Self(Arc::new(ClimateNoiseGenerator::new(0)))
    }
}

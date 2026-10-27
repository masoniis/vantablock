use crate::prelude::*;
use crate::world::terrain::shaping::{RealisticShaper, SimplexShaper};
use crate::world::terrain::{
    ActiveTerrainGenerator, NoisyShaper, SinwaveShaper, SuperflatShaper, TerrainShaper,
};
use bevy::ecs::{
    resource::Resource,
    system::{Local, Res, ResMut},
    world::{FromWorld, World},
};
use std::sync::Arc;

#[derive(Resource)]
pub struct TerrainGeneratorLibrary {
    pub generators: Vec<Arc<dyn TerrainShaper + Send + Sync>>,
}

impl FromWorld for TerrainGeneratorLibrary {
    fn from_world(_world: &mut World) -> Self {
        Self {
            generators: vec![
                Arc::new(SuperflatShaper::default()),
                Arc::new(SinwaveShaper::default()),
                Arc::new(NoisyShaper::new()),
                Arc::new(RealisticShaper::new()),
                Arc::new(SimplexShaper::new()),
            ],
        }
    }
}

/// A simple startup system that sets the default terrain generator to avoid confusion
/// regarding the default state of the `ActiveTerrainGenerator` resource.
pub fn set_default_terrain_generator(
    mut active_generator: ResMut<ActiveTerrainGenerator>,
    library: Res<TerrainGeneratorLibrary>,
) {
    active_generator.0 = library.generators[0].clone();
}

/// A simple system that cycles through terran generators (shapers).
pub fn cycle_active_generator(
    mut active_generator: ResMut<ActiveTerrainGenerator>,
    library: Res<TerrainGeneratorLibrary>,
    mut current_index: Local<usize>,
) {
    *current_index = (*current_index + 1) % library.generators.len();
    active_generator.0 = library.generators[*current_index].clone();

    info!("Switched to generator index: {}", *current_index);
}

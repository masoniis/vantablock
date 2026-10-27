use crate::world::terrain::generators::painting::{
    lib::TerrainPainter, simple_surface_painter::SimpleSurfacePainter,
};
use bevy::ecs::prelude::Resource;
use std::sync::Arc;

/// A resource holding the active terrain chunk painter.
#[derive(Resource, Clone)]
pub struct ActiveTerrainPainter(pub Arc<dyn TerrainPainter + Send + Sync>);

impl Default for ActiveTerrainPainter {
    fn default() -> Self {
        Self(Arc::new(SimpleSurfacePainter::new()))
    }
}

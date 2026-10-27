use crate::world::terrain::shaping::{RealisticShaper, SimplexShaper};
use crate::world::terrain::{NoisyShaper, SinwaveShaper, SuperflatShaper, TerrainShaper};
use bevy::ecs::resource::Resource;
use common::world::terrain::TerrainShaperKind;
use std::sync::Arc;

/// Every terrain shaper the server can switch to, built once so switching back reuses it.
#[derive(Resource)]
pub struct TerrainGeneratorLibrary {
    superflat: Arc<dyn TerrainShaper + Send + Sync>,
    sinwave: Arc<dyn TerrainShaper + Send + Sync>,
    noisy: Arc<dyn TerrainShaper + Send + Sync>,
    realistic: Arc<dyn TerrainShaper + Send + Sync>,
    simplex3d: Arc<dyn TerrainShaper + Send + Sync>,
}

impl Default for TerrainGeneratorLibrary {
    fn default() -> Self {
        Self {
            superflat: Arc::new(SuperflatShaper::default()),
            sinwave: Arc::new(SinwaveShaper::default()),
            noisy: Arc::new(NoisyShaper::new()),
            realistic: Arc::new(RealisticShaper::new()),
            simplex3d: Arc::new(SimplexShaper::new()),
        }
    }
}

impl TerrainGeneratorLibrary {
    /// The shaper of this kind.
    pub fn get(&self, kind: TerrainShaperKind) -> &Arc<dyn TerrainShaper + Send + Sync> {
        match kind {
            TerrainShaperKind::Superflat => &self.superflat,
            TerrainShaperKind::Sinwave => &self.sinwave,
            TerrainShaperKind::Noisy => &self.noisy,
            TerrainShaperKind::Realistic => &self.realistic,
            TerrainShaperKind::Simplex3d => &self.simplex3d,
        }
    }
}

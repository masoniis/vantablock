use crate::simulation_world::chunk::CHUNK_SIDE_LENGTH;
use std::cell::RefCell;

/// A thread-local pool of buffers to prevent re-allocating buffers every chunk.
pub struct ClimateBufferPool {
    pub temperature: Vec<f32>,
    pub precipitation: Vec<f32>,
    pub continentalness: Vec<f32>,
    pub erosion: Vec<f32>,
    pub weirdness: Vec<f32>,
}

impl Default for ClimateBufferPool {
    fn default() -> Self {
        let cap = CHUNK_SIDE_LENGTH * CHUNK_SIDE_LENGTH;
        Self {
            temperature: vec![0.0; cap],
            precipitation: vec![0.0; cap],
            continentalness: vec![0.0; cap],
            erosion: vec![0.0; cap],
            weirdness: vec![0.0; cap],
        }
    }
}

impl ClimateBufferPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resizes buffers if needed (e.g. LOD change) without re-allocating if capacity allows.
    pub fn prepare(&mut self, size: usize) {
        let len = size * size;
        // TODO: perhaps use less safe set_len for speed gains instead of resize
        if self.temperature.len() < len {
            self.temperature.resize(len, 0.0);
            self.precipitation.resize(len, 0.0);
            self.continentalness.resize(len, 0.0);
            self.erosion.resize(len, 0.0);
            self.weirdness.resize(len, 0.0);
        }
    }
}

thread_local! {
    pub static CLIMATE_BUFFERS: RefCell<ClimateBufferPool> = RefCell::new(ClimateBufferPool::new());
}

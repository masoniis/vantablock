use shared::world::chunk::CHUNK_SIDE_LENGTH;
use std::cell::RefCell;

const MAX_BUFFER_LEN: usize = CHUNK_SIDE_LENGTH * CHUNK_SIDE_LENGTH;

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
        Self {
            temperature: vec![0.0; MAX_BUFFER_LEN],
            precipitation: vec![0.0; MAX_BUFFER_LEN],
            continentalness: vec![0.0; MAX_BUFFER_LEN],
            erosion: vec![0.0; MAX_BUFFER_LEN],
            weirdness: vec![0.0; MAX_BUFFER_LEN],
        }
    }
}

impl ClimateBufferPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns mutable sub-slices of the exact requested length from the pre-allocated buffers.
    pub fn get_slices(
        &mut self,
        size: usize,
    ) -> (&mut [f32], &mut [f32], &mut [f32], &mut [f32], &mut [f32]) {
        let len = size * size;
        debug_assert!(len <= MAX_BUFFER_LEN);

        (
            &mut self.temperature[..len],
            &mut self.precipitation[..len],
            &mut self.continentalness[..len],
            &mut self.erosion[..len],
            &mut self.weirdness[..len],
        )
    }
}

thread_local! {
    pub static CLIMATE_BUFFERS: RefCell<ClimateBufferPool> = RefCell::new(ClimateBufferPool::new());
}

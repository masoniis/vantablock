use super::padded_chunk_view::PADDED_SIZE;
use crate::prelude::*;
use crate::simulation::block::BlockId;
use std::cell::RefCell;

pub const TOTAL_BUFFER_SIZE: usize = PADDED_SIZE * PADDED_SIZE * PADDED_SIZE;

thread_local! {
    pub static BLOCKID_BUFFER_POOL: RefCell<Vec<Vec<BlockId>>> = const { RefCell::new(Vec::new()) };
}

/// Get a buffer from the thread buffer pool
pub fn acquire_buffer() -> Vec<BlockId> {
    BLOCKID_BUFFER_POOL.with(|pool| match pool.borrow_mut().pop() {
        Some(mut vec) => {
            vec.clear();
            vec
        }
        None => {
            if let Some(index) = rayon::current_thread_index() {
                debug!(
                    target : "memory",
                    "Allocating a new vector in thread buffer pool for Worker {}",
                    index
                );
            } else {
                debug!("Allocating a new vector on Main Thread");
            }
            Vec::with_capacity(TOTAL_BUFFER_SIZE)
        }
    })
}

/// Release a buffer back to the buffer pool.
pub fn release_buffer(vec: Vec<BlockId>) {
    BLOCKID_BUFFER_POOL.with(|pool| {
        pool.borrow_mut().push(vec);
    })
}

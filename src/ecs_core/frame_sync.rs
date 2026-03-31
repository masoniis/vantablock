use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug)]
pub struct Inner {
    // A mutex to protect the shared state from concurrent access.
    mutex: Mutex<FrameState>,
    // A condvar to allow threads to wait efficiently for the state to change.
    condvar: Condvar,
}

// The shared state that the threads will synchronize on.
#[derive(Debug, PartialEq, Eq)]
enum FrameState {
    Simulating, // it's the simulation thread's turn.
    Rendering,  // it's the render thread's turn.
}

/// A synchronization primitive to coordinate simulation and render threads
/// using a Mutex and a Condvar.
#[derive(Debug, Clone)]
pub struct FrameSync {
    inner: Arc<Inner>,
}

impl Default for FrameSync {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameSync {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                mutex: Mutex::new(FrameState::Simulating),
                condvar: Condvar::new(),
            }),
        }
    }

    /// Called by the simulation thread to wait for its turn.
    /// It will block until the render thread calls `finish_rendering`.
    pub fn wait_for_simulation(&self) {
        let mut state = self.inner.mutex.lock().unwrap();
        // crucial loop to handle "spuriouss wakeups"
        while *state != FrameState::Simulating {
            state = self.inner.condvar.wait(state).unwrap();
        }
    }

    /// Called by the simulation thread after it has finished its work for the frame.
    /// This changes the state and notifies the render thread that it can begin.
    pub fn finish_simulation(&self) {
        let mut state = self.inner.mutex.lock().unwrap();
        *state = FrameState::Rendering;
        self.inner.condvar.notify_one();
    }

    /// Called by the render thread to wait for its turn.
    /// It will block until the simulation thread calls `finish_simulation`.
    pub fn wait_for_extraction(&self) {
        let mut state = self.inner.mutex.lock().unwrap();
        while *state != FrameState::Rendering {
            state = self.inner.condvar.wait(state).unwrap();
        }
    }

    /// Called by the render thread after it has finished rendering the frame.
    /// This changes the state and notifies the simulation thread that it can start the next frame.
    pub fn finish_extraction(&self) {
        let mut state = self.inner.mutex.lock().unwrap();
        *state = FrameState::Simulating;
        self.inner.condvar.notify_one();
    }
}

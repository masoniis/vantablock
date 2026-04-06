use bevy::ecs::prelude::*;
use bevy::state::state::States;
use std::marker::PhantomData;

/// A temporary resource that tells the master finalizer system which state
/// to transition to when the current loading process is complete.
///
/// It is generic over the state machine it should operate on (e.g., AppState).
#[derive(Resource, Debug)]
pub struct OnLoadComplete<S: States> {
    /// The destination state for the transition.
    pub destination: S,
    /// A marker to make the compiler happy with the generic type `S`.
    _marker: PhantomData<S>,
}

impl<S: States> OnLoadComplete<S> {
    pub fn new(destination: S) -> Self {
        Self {
            destination,
            _marker: PhantomData,
        }
    }
}

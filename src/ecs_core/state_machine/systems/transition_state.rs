use crate::ecs_core::state_machine::{
    resources::{CurrentState, NextState},
    State,
};
use crate::ecs_core::worlds::SimulationWorldMarker;
use crate::prelude::*;
pub use crate::simulation_world::scheduling::{OnEnter, OnExit};
use bevy::ecs::prelude::*;
use std::any::type_name;

#[instrument(skip_all)]
pub fn apply_state_transition_system<T: State>(world: &mut World) {
    // Decide if a transition should occur or not
    let transition = {
        let Some(mut next_state_res) = world.get_resource_mut::<NextState<T>>() else {
            return; // no NextState resource exists, so nothing to do.
        };

        let Some(new_state) = next_state_res.val.take() else {
            return; // `NextState` resource exists, but no state is pending.
        };

        let old_state = world.resource::<CurrentState<T>>().val;

        // if the states are the same return none, otherwise return the transition
        if old_state == new_state {
            None
        } else {
            Some((old_state, new_state))
        }
    };

    // apply transitions
    if let Some((old_state, new_state)) = transition {
        let is_simulation_world = world.get_resource::<SimulationWorldMarker>().is_some();
        if is_simulation_world {
            let state_type_name = type_name::<T>().split("::").last().unwrap_or_default();
            info!(
                "\n\nState Transition ({}): {:?} -> {:?}\n",
                state_type_name, old_state, new_state
            );
        }

        let curr_world = if is_simulation_world {
            "simulation"
        } else {
            "render"
        };

        // run the OnExit schedule for the old state.
        if let Err(e) = world.try_run_schedule(OnExit(old_state)) {
            debug!(target: "missing_transitions", "({} world) {}", curr_world, e);
        }

        // update the CurrentState resource with the new state.
        let mut current_state_res = world.resource_mut::<CurrentState<T>>();
        current_state_res.val = new_state;

        // run the OnEnter schedule for the new state.
        if let Err(e) = world.try_run_schedule(OnEnter(new_state)) {
            debug!(target: "missing_transitions", "({} world) {}", curr_world, e);
        }
    }
}

use crate::ecs_core::state_machine::resources::{CurrentState, NextState};
use crate::ecs_core::state_machine::State;
use crate::prelude::*;
use crate::render_world::global_extract::utils::run_extract_schedule::SimulationWorld;
use bevy::ecs::prelude::*;

/// Detects a state change in the simulation_world and queues a
/// corresponding state change in the render_world.
#[instrument(skip_all)]
pub fn extract_state_system<T: State>(
    // Input
    simulation_world: Res<SimulationWorld>,
    render_world_state: Res<CurrentState<T>>,

    // Output next state
    mut next_state: ResMut<NextState<T>>,
) {
    let simulation_world_state = simulation_world
        .val
        .get_resource::<CurrentState<T>>()
        .unwrap();

    // If the simulation_world has a state that the render_world doesn't have yet...
    if simulation_world_state.val != render_world_state.val {
        debug!(
            target: "state_machine",
            "Render world extracted a state change: {:?} -> {:?}",
            render_world_state.val, simulation_world_state.val
        );
        next_state.val = Some(simulation_world_state.val);
    }
}

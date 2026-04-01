use crate::prelude::*;
use crate::render_world::global_extract::utils::run_extract_schedule::SimulationWorld;
use bevy::ecs::prelude::*; // Your path might differ

/// A custom run condition that returns `true` if a resource of type `T` has been added
/// or changed in the **game world** since the last time this condition was checked.
pub fn simulation_world_resource_changed<T: Resource>(
    simulation_world: Res<SimulationWorld>,
) -> bool {
    let world = &simulation_world.val;

    // Check if the resource exists and if its "changed" flag is set.
    let is_changed = world
        .get_resource::<T>()
        .is_some_and(|_| world.is_resource_changed::<T>());

    if is_changed {
        debug!(
            target: "simulation_world_resource_changed",
            "Resource of type {} changed.",
            std::any::type_name::<T>(),
        );
    }

    is_changed
}

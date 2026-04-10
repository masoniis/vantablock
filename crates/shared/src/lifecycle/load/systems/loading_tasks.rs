use crate::{lifecycle::load::components::LoadingTaskComponent, prelude::*};
use bevy::{
    ecs::prelude::*,
    prelude::{NextState, States},
    state::state::FreelyMutableState,
    tasks::{block_on, poll_once},
};

/// A generic polling system that checks if tasks of a specific marker have finished.
///
/// When a task finishes, this system applies the tasks' returned commands and despawns
/// the task entity.
pub fn poll_tasks<Marker: Component>(
    mut tasks: Query<(Entity, &mut LoadingTaskComponent), With<Marker>>,
    mut commands: Commands,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(mut queue) = block_on(poll_once(&mut task.0)) {
            commands.append(&mut queue);
            commands.entity(entity).despawn();
        }
    }
}

/// A generic cleanup system for orphaned tasks.
pub fn cleanup_orphaned_tasks<Marker: Component>(
    mut commands: Commands,
    query: Query<Entity, (With<LoadingTaskComponent>, With<Marker>)>,
) {
    for entity in &query {
        warn!(
            "Cleaning up orphaned task for marker: {}",
            std::any::type_name::<Marker>()
        );
        commands.entity(entity).despawn();
    }
}

/// Returns true if there are no entities with the given Marker component.
///
/// On debug builds, prints a warning if it returns true on the first call
/// which is useful to catch accidental race conditions or useless polling.
pub fn loading_is_complete<Marker: Component>(query: Query<(), With<Marker>>) -> bool {
    query.is_empty()
}

/// A transition checker that moves to a target state when all entities with a marker are gone.
pub fn check_loading_complete<Marker: Component, S: States + Copy + FreelyMutableState>(
    target_state: S,
) -> impl FnMut(Query<(), With<Marker>>, ResMut<NextState<S>>) {
    move |tasks_query, mut next_state| {
        if tasks_query.is_empty() {
            next_state.set(target_state);
        }
    }
}

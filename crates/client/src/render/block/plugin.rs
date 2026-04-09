use super::meshing::{poll_chunk_meshing_tasks, start_pending_meshing_tasks_system};
use bevy::app::{App, FixedUpdate, Plugin};
use bevy::prelude::IntoScheduleConfigs;
use shared::simulation::scheduling::FixedUpdateSet;

pub struct BlockRenderPlugin;

impl Plugin for BlockRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (start_pending_meshing_tasks_system, poll_chunk_meshing_tasks)
                .in_set(FixedUpdateSet::MainLogic),
        );
    }
}

use crate::{
    prelude::*,
    render::{
        block::BlockRenderDataRegistry,
        chunk::{
            ChunkMesh,
            tasks::task::{ChunkMeshingTaskComponent, MeshingInput},
        },
    },
    world::chunk::{
        ClientChunkMap,
        range::{StreamingAnchorQuery, is_within_horizontal_range, streaming_center},
    },
};
use bevy::{ecs::prelude::*, tasks::AsyncComputeTaskPool};
use common::world::{
    block::BlockRegistry,
    chunk::{ChunkBlocksComponent, ChunkCoord, RENDER_DISTANCE},
};

/// How many meshing tasks may be in flight per compute thread. Keeping the pool's queue this
/// short is what makes the nearest chunks go first: work queued early can't be reprioritized
/// when the player moves.
const TASKS_PER_THREAD: usize = 2;

/// Decides which chunks to mesh, from scratch every frame.
///
/// It queues loaded chunks that entered render distance, drops queued ones that left it, then
/// hands each free task slot to the nearest queued chunk whose neighbours all have data. A
/// chunk whose neighbours are still missing is simply checked again next frame.
#[instrument(skip_all)]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn schedule_chunk_meshing_system(
    // input
    anchor: StreamingAnchorQuery,
    chunk_map: Res<ClientChunkMap>,
    blocks: Query<&ChunkBlocksComponent>,
    block_registry: Res<BlockRegistry>,
    render_registry: Res<BlockRenderDataRegistry>,

    // output
    mut chunks: Query<
        (
            Entity,
            &ChunkCoord,
            Option<&mut ChunkMesh>,
            Has<ChunkMeshingTaskComponent>,
        ),
        With<ChunkBlocksComponent>,
    >,
    mut commands: Commands,
) {
    let Some(center) = streaming_center(&anchor) else {
        return;
    };

    // queue chunks that entered render distance, un-queue ones that left it
    for (entity, coord, mesh, has_task) in &chunks {
        let in_range = is_within_horizontal_range(center, coord.pos, RENDER_DISTANCE);
        let current = mesh.copied();
        match (current, next_mesh_state(current, in_range, has_task)) {
            (None, Some(state)) => {
                commands.entity(entity).insert(state);
            }
            (Some(_), None) => {
                commands.entity(entity).remove::<ChunkMesh>();
            }
            _ => {}
        }
    }

    // hand free task slots to the nearest queued chunks that can mesh now
    let max_tasks = AsyncComputeTaskPool::get().thread_num().max(1) * TASKS_PER_THREAD;
    let in_flight = chunks.iter().filter(|(.., has_task)| *has_task).count();
    let mut free_slots = max_tasks.saturating_sub(in_flight);

    let queued = chunks.iter().filter_map(|(entity, coord, mesh, has_task)| {
        let queued = mesh.is_some_and(|mesh| *mesh == ChunkMesh::Queued) && !has_task;
        // an entity the chunk map no longer tracks is being unloaded
        let tracked = chunk_map.get_entity(coord.pos) == Some(entity);
        (queued && tracked).then_some((coord.pos, entity))
    });
    let candidates = nearest_first(queued, center);

    for (coord, entity) in candidates {
        if free_slots == 0 {
            break;
        }
        let Ok(chunk) = blocks.get(entity) else {
            continue;
        };
        let Some(input) = MeshingInput::gather(coord, chunk, &chunk_map, &blocks) else {
            continue;
        };

        trace!(target: "chunk_loading", "Starting meshing task for {coord}");
        commands
            .entity(entity)
            .insert(input.spawn(&block_registry, &render_registry));
        if let Ok((.., Some(mut mesh), _)) = chunks.get_mut(entity) {
            *mesh = ChunkMesh::Meshing;
        }
        free_slots -= 1;
    }
}

/// The mesh state a loaded chunk should have this frame, from its current one.
///
/// A chunk in render distance without a state is queued; a queued chunk that left render
/// distance is un-queued. A chunk with a task in flight is left alone, since `poll` settles it,
/// and a meshed chunk keeps its mesh when it leaves.
pub fn next_mesh_state(
    current: Option<ChunkMesh>,
    in_range: bool,
    has_task: bool,
) -> Option<ChunkMesh> {
    match current {
        None if in_range => Some(ChunkMesh::Queued),
        Some(ChunkMesh::Queued) if !in_range && !has_task => None,
        other => other,
    }
}

/// Orders chunks nearest to `center` first. Ties are broken by coordinate so the order never
/// depends on query or hash map iteration order.
pub fn nearest_first(
    chunks: impl IntoIterator<Item = (IVec3, Entity)>,
    center: IVec3,
) -> Vec<(IVec3, Entity)> {
    let mut chunks: Vec<_> = chunks.into_iter().collect();
    chunks
        .sort_unstable_by_key(|(coord, _)| ((*coord - center).length_squared(), coord.to_array()));
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: a queued chunk that left render distance stayed queued forever, waiting on
    /// neighbours that had been unloaded, so the meshing queue never drained.
    #[test]
    fn queued_chunk_leaving_render_distance_is_unqueued() {
        assert_eq!(next_mesh_state(Some(ChunkMesh::Queued), false, false), None);
    }

    #[test]
    fn chunk_entering_render_distance_is_queued() {
        assert_eq!(next_mesh_state(None, true, false), Some(ChunkMesh::Queued));
        assert_eq!(next_mesh_state(None, false, false), None);
    }

    /// A chunk edited mid-mesh is `Queued` with a task still running; un-queueing it would
    /// lose the remesh, so it waits for `poll`.
    #[test]
    fn queued_chunk_with_a_task_in_flight_stays_queued() {
        assert_eq!(
            next_mesh_state(Some(ChunkMesh::Queued), false, true),
            Some(ChunkMesh::Queued)
        );
    }

    #[test]
    fn meshed_chunk_keeps_its_mesh_out_of_range() {
        assert_eq!(
            next_mesh_state(Some(ChunkMesh::Meshed), false, false),
            Some(ChunkMesh::Meshed)
        );
    }

    #[test]
    fn candidates_are_nearest_first_with_a_stable_tiebreak() {
        let entity = |i| Entity::from_raw_u32(i).unwrap();
        let order: Vec<IVec3> = nearest_first(
            [
                (IVec3::new(5, 0, 0), entity(1)),
                (IVec3::new(0, 0, 3), entity(2)),
                (IVec3::new(1, 0, 0), entity(3)),
                (IVec3::new(-1, 0, 0), entity(4)),
            ],
            IVec3::ZERO,
        )
        .into_iter()
        .map(|(coord, _)| coord)
        .collect();

        assert_eq!(
            order,
            [
                IVec3::new(-1, 0, 0),
                IVec3::new(1, 0, 0),
                IVec3::new(0, 0, 3),
                IVec3::new(5, 0, 0)
            ]
        );
    }
}

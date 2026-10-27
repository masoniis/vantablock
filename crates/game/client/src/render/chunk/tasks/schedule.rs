use crate::{
    prelude::*,
    render::{
        block::BlockRenderDataRegistry,
        chunk::tasks::task::{ChunkMeshingTaskComponent, MeshingInput},
    },
    world::chunk::{
        ClientChunkMap, ClientChunkState,
        range::{StreamingAnchorQuery, is_within_horizontal_range, streaming_center},
    },
};
use bevy::{ecs::prelude::*, tasks::AsyncComputeTaskPool};
use common::world::{
    block::BlockRegistry,
    chunk::{ChunkBlocksComponent, RENDER_DISTANCE},
};

/// How many meshing tasks may be in flight per compute thread. Keeping the pool's queue this
/// short is what makes the nearest chunks go first: work queued early can't be reprioritized
/// when the player moves.
const TASKS_PER_THREAD: usize = 2;

/// Decides which chunks to mesh, from scratch every frame.
///
/// It queues chunks that entered render distance, drops queued ones that left it, then hands
/// each free task slot to the nearest queued chunk whose neighbors all have data. A chunk
/// whose neighbors are still missing is simply checked again next frame.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn schedule_chunk_meshing_system(
    // input
    anchor: StreamingAnchorQuery,
    tasks: Query<(), With<ChunkMeshingTaskComponent>>,
    blocks: Query<&ChunkBlocksComponent>,
    block_registry: Res<BlockRegistry>,
    render_registry: Res<BlockRenderDataRegistry>,

    // output
    mut chunk_map: ResMut<ClientChunkMap>,
    mut commands: Commands,
) {
    let Some(center) = streaming_center(&anchor) else {
        return;
    };
    let has_task = |entity| tasks.contains(entity);

    sync_meshing_targets(&mut chunk_map, center, has_task);

    let max_tasks = AsyncComputeTaskPool::get().thread_num().max(1) * TASKS_PER_THREAD;
    let mut free_slots = max_tasks.saturating_sub(tasks.iter().count());

    for (coord, entity) in meshing_candidates(&chunk_map, center, has_task) {
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
        chunk_map.mark_as_meshing(coord, entity);
        free_slots -= 1;
    }
}

/// Queues chunks with data that are within render distance of `center`, and un-queues
/// queued chunks that left it. A chunk with a task in flight is left alone: `poll` settles it.
pub fn sync_meshing_targets(
    chunk_map: &mut ClientChunkMap,
    center: IVec3,
    has_task: impl Fn(Entity) -> bool,
) {
    for (&coord, state) in chunk_map.chunk_states.iter_mut() {
        let in_range = is_within_horizontal_range(center, coord, RENDER_DISTANCE);
        match *state {
            ClientChunkState::DataReady { entity } if in_range => {
                *state = ClientChunkState::NeedsMeshing { entity };
            }
            ClientChunkState::NeedsMeshing { entity } if !in_range && !has_task(entity) => {
                *state = ClientChunkState::DataReady { entity };
            }
            _ => {}
        }
    }
}

/// The queued chunks that can start a task (none already in flight), nearest to `center`
/// first. Ties are broken by coordinate so the order never depends on hash map iteration.
pub fn meshing_candidates(
    chunk_map: &ClientChunkMap,
    center: IVec3,
    has_task: impl Fn(Entity) -> bool,
) -> Vec<(IVec3, Entity)> {
    let mut candidates: Vec<(IVec3, Entity)> = chunk_map
        .chunk_states
        .iter()
        .filter_map(|(&coord, state)| match *state {
            ClientChunkState::NeedsMeshing { entity } if !has_task(entity) => Some((coord, entity)),
            _ => None,
        })
        .collect();
    candidates
        .sort_unstable_by_key(|(coord, _)| ((*coord - center).length_squared(), coord.to_array()));
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(index: u32) -> Entity {
        Entity::from_raw_u32(index).unwrap()
    }

    fn chunk_map_with(states: &[(IVec3, ClientChunkState)]) -> ClientChunkMap {
        let mut chunk_map = ClientChunkMap::default();
        chunk_map.chunk_states.extend(states.iter().copied());
        chunk_map
    }

    /// Regression: a queued chunk that left render distance stayed queued forever, waiting
    /// on neighbors that had been unloaded, so the meshing queue never drained.
    #[test]
    fn queued_chunk_leaving_render_distance_is_unqueued() {
        let far = IVec3::new(RENDER_DISTANCE + 1, 0, 0);
        let near = IVec3::new(1, 0, 0);
        let mut chunk_map = chunk_map_with(&[
            (far, ClientChunkState::NeedsMeshing { entity: entity(1) }),
            (near, ClientChunkState::DataReady { entity: entity(2) }),
        ]);

        sync_meshing_targets(&mut chunk_map, IVec3::ZERO, |_| false);

        assert_eq!(
            chunk_map.get_state(far),
            Some(ClientChunkState::DataReady { entity: entity(1) })
        );
        assert_eq!(
            chunk_map.get_state(near),
            Some(ClientChunkState::NeedsMeshing { entity: entity(2) })
        );
    }

    /// A chunk edited mid-mesh is `NeedsMeshing` with a task still running; un-queueing it
    /// would lose the remesh, so it waits for `poll`.
    #[test]
    fn queued_chunk_with_a_task_in_flight_stays_queued() {
        let far = IVec3::new(RENDER_DISTANCE + 1, 0, 0);
        let mut chunk_map =
            chunk_map_with(&[(far, ClientChunkState::NeedsMeshing { entity: entity(1) })]);

        sync_meshing_targets(&mut chunk_map, IVec3::ZERO, |_| true);

        assert_eq!(
            chunk_map.get_state(far),
            Some(ClientChunkState::NeedsMeshing { entity: entity(1) })
        );
    }

    #[test]
    fn candidates_are_nearest_first_and_skip_chunks_in_flight() {
        let busy = entity(4);
        let chunk_map = chunk_map_with(&[
            (
                IVec3::new(5, 0, 0),
                ClientChunkState::NeedsMeshing { entity: entity(1) },
            ),
            (
                IVec3::new(1, 0, 0),
                ClientChunkState::NeedsMeshing { entity: entity(2) },
            ),
            (
                IVec3::new(0, 0, 3),
                ClientChunkState::NeedsMeshing { entity: entity(3) },
            ),
            (
                IVec3::new(0, 0, 0),
                ClientChunkState::NeedsMeshing { entity: busy },
            ),
            (
                IVec3::new(0, 1, 0),
                ClientChunkState::MeshComplete { entity: entity(5) },
            ),
        ]);

        let order: Vec<IVec3> = meshing_candidates(&chunk_map, IVec3::ZERO, |e| e == busy)
            .into_iter()
            .map(|(coord, _)| coord)
            .collect();

        assert_eq!(
            order,
            [
                IVec3::new(1, 0, 0),
                IVec3::new(0, 0, 3),
                IVec3::new(5, 0, 0)
            ]
        );
    }
}

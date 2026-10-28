use crate::{
    prelude::*,
    render::chunk::{
        BlockMeshAsset, ChunkMesh, OpaqueMeshComponent, TransparentMeshComponent,
        meshing::{OpaqueMeshData, TransparentMeshData},
        tasks::task::ChunkMeshingTaskComponent,
    },
};
use bevy::{asset::Assets, ecs::prelude::*, prelude::*};
use common::world::chunk::{CHUNK_SIDE_LENGTH, ChunkCoord};
use crossbeam::channel::TryRecvError;

/// Collects finished meshing tasks and attaches their meshes to the chunk entities.
///
/// The task component is removed as soon as a task finishes, whatever the chunk's state, so
/// its slot is always freed for the scheduler.
#[instrument(skip_all)]
pub fn poll_chunk_meshing_tasks(
    // input
    mut tasks_query: Query<(
        Entity,
        &ChunkMeshingTaskComponent,
        &ChunkCoord,
        Option<&mut ChunkMesh>,
    )>,

    // output
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<BlockMeshAsset>>,
) {
    for (entity, task, coord, mesh) in &mut tasks_query {
        let meshes = match task.receiver.try_recv() {
            Ok(meshes) => Some(meshes),
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => None,
        };
        commands
            .entity(entity)
            .remove::<ChunkMeshingTaskComponent>();

        let Some(mut mesh) = mesh else {
            debug!(target: "chunk_loading", "Discarding the mesh of {coord}, which is no longer wanted");
            continue;
        };
        match (*mesh, meshes) {
            (ChunkMesh::Meshing, Some(meshes)) => {
                trace!(target: "chunk_loading", "Chunk meshing finished for {coord}");
                attach_meshes(&mut commands, &mut mesh_assets, entity, coord, meshes);
                *mesh = ChunkMesh::Meshed;
            }
            // an edit landed while the task ran: show this (now stale) mesh meanwhile, and keep
            // the chunk queued so the scheduler remeshes it with the edit
            (ChunkMesh::Queued, Some(meshes)) => {
                attach_meshes(&mut commands, &mut mesh_assets, entity, coord, meshes);
            }
            (ChunkMesh::Meshing, None) => {
                warn!(target: "chunk_loading", "Meshing task for {coord} failed, queueing it again");
                *mesh = ChunkMesh::Queued;
            }
            _ => {}
        }
    }
}

/// Replaces the chunk's mesh components with the new meshes, placed at the chunk's origin.
fn attach_meshes(
    commands: &mut Commands,
    mesh_assets: &mut Assets<BlockMeshAsset>,
    entity: Entity,
    coord: &ChunkCoord,
    (opaque, transparent): (Option<OpaqueMeshData>, Option<TransparentMeshData>),
) {
    let mut chunk = commands.entity(entity);
    chunk.remove::<(OpaqueMeshComponent, TransparentMeshComponent)>();

    if opaque.is_none() && transparent.is_none() {
        // nothing visible, so nothing to place
        return;
    }
    if let Some(mesh) = opaque {
        chunk.insert(OpaqueMeshComponent::new(mesh_assets.add(mesh)));
    }
    if let Some(mesh) = transparent {
        chunk.insert(TransparentMeshComponent::new(mesh_assets.add(mesh)));
    }
    chunk.insert((
        Transform::from_translation((coord.pos * CHUNK_SIDE_LENGTH as i32).as_vec3()),
        Visibility::default(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::unbounded;

    /// Runs `poll_chunk_meshing_tasks` once for a chunk whose in-flight task returns `result`,
    /// with its mesh state already `state`, and returns the entity's mesh state afterwards.
    fn poll_with_state(
        state: Option<ChunkMesh>,
        result: (Option<OpaqueMeshData>, Option<TransparentMeshData>),
    ) -> (Option<ChunkMesh>, bool) {
        let mut app = App::new();
        app.init_resource::<Assets<BlockMeshAsset>>();
        app.add_systems(Update, poll_chunk_meshing_tasks);

        let (sender, receiver) = unbounded();
        sender.send(result).unwrap();
        let mut chunk = app.world_mut().spawn((
            ChunkCoord {
                pos: IVec3::new(2, 1, 3),
            },
            ChunkMeshingTaskComponent { receiver },
        ));
        if let Some(state) = state {
            chunk.insert(state);
        }
        let entity = chunk.id();

        app.update();

        let chunk = app.world().entity(entity);
        (
            chunk.get::<ChunkMesh>().copied(),
            chunk.contains::<ChunkMeshingTaskComponent>(),
        )
    }

    fn some_mesh() -> OpaqueMeshData {
        BlockMeshAsset {
            name: "test".into(),
            faces: vec![crate::render::chunk::meshing::PackedFace(0)].into(),
        }
    }

    #[test]
    fn finished_mesh_completes_a_meshing_chunk() {
        for result in [(None, None), (Some(some_mesh()), None)] {
            let (state, _) = poll_with_state(Some(ChunkMesh::Meshing), result);
            assert_eq!(state, Some(ChunkMesh::Meshed));
        }
    }

    /// Regression: an edit during meshing queues the chunk again. The finished (stale) mesh used
    /// to overwrite that with "complete", so the edit never got remeshed.
    #[test]
    fn edit_during_meshing_keeps_the_remesh_queued() {
        for result in [(None, None), (Some(some_mesh()), None)] {
            let (state, _) = poll_with_state(Some(ChunkMesh::Queued), result);
            assert_eq!(state, Some(ChunkMesh::Queued));
        }
    }

    /// A task whose chunk was un-queued meanwhile must still give up its slot, or the
    /// scheduler's in-flight count grows until meshing stops.
    #[test]
    fn finished_task_always_frees_its_slot() {
        let (state, has_task) = poll_with_state(None, (Some(some_mesh()), None));
        assert_eq!(state, None);
        assert!(!has_task);
    }
}

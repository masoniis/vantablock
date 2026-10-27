use crate::{
    prelude::*,
    render::chunk::{
        BlockMeshAsset, OpaqueMeshComponent, TransparentMeshComponent,
        meshing::{OpaqueMeshData, TransparentMeshData},
        tasks::task::ChunkMeshingTaskComponent,
    },
    world::chunk::{ClientChunkMap, ClientChunkState},
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
    tasks_query: Query<(Entity, &ChunkMeshingTaskComponent, &ChunkCoord)>,

    // output
    mut commands: Commands,
    mut chunk_map: ResMut<ClientChunkMap>,
    mut mesh_assets: ResMut<Assets<BlockMeshAsset>>,
) {
    for (entity, task, coord) in tasks_query.iter() {
        let meshes = match task.receiver.try_recv() {
            Ok(meshes) => Some(meshes),
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => None,
        };
        commands
            .entity(entity)
            .remove::<ChunkMeshingTaskComponent>();

        match (chunk_map.get_state(coord.pos), meshes) {
            (Some(ClientChunkState::Meshing { entity: tracked }), Some(meshes))
                if tracked == entity =>
            {
                trace!(target: "chunk_loading", "Chunk meshing finished for {coord}");
                attach_meshes(&mut commands, &mut mesh_assets, entity, coord, meshes);
                chunk_map.mark_as_mesh_complete(coord.pos, entity);
            }
            // an edit landed while the task ran: show this (now stale) mesh meanwhile, and keep
            // the chunk queued so the scheduler remeshes it with the edit
            (Some(ClientChunkState::NeedsMeshing { entity: tracked }), Some(meshes))
                if tracked == entity =>
            {
                attach_meshes(&mut commands, &mut mesh_assets, entity, coord, meshes);
            }
            (Some(ClientChunkState::Meshing { entity: tracked }), None) if tracked == entity => {
                warn!(target: "chunk_loading", "Meshing task for {coord} failed, queueing it again");
                chunk_map.mark_as_needs_meshing(coord.pos, entity);
            }
            _ => {
                debug!(target: "chunk_loading", "Discarding the mesh of {coord}, which is no longer tracked");
            }
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
    /// with the chunk map already in `state`, and returns the state afterwards.
    fn poll_with_state(
        state: fn(Entity) -> ClientChunkState,
        result: (Option<OpaqueMeshData>, Option<TransparentMeshData>),
    ) -> ClientChunkState {
        let coord = IVec3::new(2, 1, 3);
        let mut app = App::new();
        app.init_resource::<ClientChunkMap>();
        app.init_resource::<Assets<BlockMeshAsset>>();
        app.add_systems(Update, poll_chunk_meshing_tasks);

        let (sender, receiver) = unbounded();
        sender.send(result).unwrap();
        let entity = app
            .world_mut()
            .spawn((
                ChunkCoord { pos: coord },
                ChunkMeshingTaskComponent { receiver },
            ))
            .id();
        app.world_mut()
            .resource_mut::<ClientChunkMap>()
            .chunk_states
            .insert(coord, state(entity));

        app.update();

        app.world()
            .resource::<ClientChunkMap>()
            .get_state(coord)
            .unwrap()
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
            let state = poll_with_state(|entity| ClientChunkState::Meshing { entity }, result);
            assert!(matches!(state, ClientChunkState::MeshComplete { .. }));
        }
    }

    /// Regression: an edit during meshing marks the chunk `NeedsMeshing`. The finished (stale)
    /// mesh used to overwrite that with `MeshComplete`, so the edit never got remeshed.
    #[test]
    fn edit_during_meshing_keeps_the_remesh_queued() {
        for result in [(None, None), (Some(some_mesh()), None)] {
            let state = poll_with_state(|entity| ClientChunkState::NeedsMeshing { entity }, result);
            assert!(matches!(state, ClientChunkState::NeedsMeshing { .. }));
        }
    }

    /// A task whose chunk was un-queued or unloaded meanwhile must still give up its slot, or
    /// the scheduler's in-flight count grows until meshing stops.
    #[test]
    fn finished_task_always_frees_its_slot() {
        let mut app = App::new();
        app.init_resource::<ClientChunkMap>();
        app.init_resource::<Assets<BlockMeshAsset>>();
        app.add_systems(Update, poll_chunk_meshing_tasks);

        let (sender, receiver) = unbounded();
        sender.send((Some(some_mesh()), None)).unwrap();
        let coord = IVec3::new(2, 1, 3);
        let entity = app
            .world_mut()
            .spawn((
                ChunkCoord { pos: coord },
                ChunkMeshingTaskComponent { receiver },
            ))
            .id();
        app.world_mut()
            .resource_mut::<ClientChunkMap>()
            .mark_as_data_ready(coord, entity);

        app.update();

        assert!(
            !app.world()
                .entity(entity)
                .contains::<ChunkMeshingTaskComponent>()
        );
    }
}

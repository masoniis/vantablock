use crate::{
    prelude::*,
    render::chunk::tasks::{CheckForMeshing, WantsMeshing},
    world::chunk::{
        ClientChunkManager, ClientChunkState,
        range::{StreamingAnchorQuery, is_within_horizontal_range, streaming_center},
    },
};
use bevy::ecs::prelude::*;
use common::world::chunk::{ChunkBlocksComponent, ChunkCoord, RENDER_DISTANCE};

/// A system that watches for newly generated chunks and promotes them to meshing if in range.
pub fn promote_newly_generated_chunks_system(
    // input
    new_data_query: Query<
        (Entity, &ChunkBlocksComponent, &ChunkCoord),
        Added<ChunkBlocksComponent>,
    >,
    anchor: StreamingAnchorQuery,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    let Some(camera_chunk_pos) = streaming_center(&anchor) else {
        return;
    };

    for (entity, _, coord) in new_data_query.iter() {
        // the manager is the source of truth: a chunk it no longer tracks was unloaded earlier
        // this frame and its despawn is still queued, so re-tracking it would leave a dead
        // entity stuck in `NeedsMeshing`
        if chunk_manager.get_entity(coord.pos) != Some(entity) {
            continue;
        }

        if is_within_horizontal_range(camera_chunk_pos, coord.pos, RENDER_DISTANCE) {
            trace!(target: "chunk_loading", "Newly generated chunk at {} is in range, promoting to WantsMeshing", coord.pos);
            commands
                .entity(entity)
                .insert((WantsMeshing, CheckForMeshing));
            chunk_manager.mark_as_needs_meshing(coord.pos, entity);
        } else {
            chunk_manager.mark_as_data_ready(coord.pos, entity);
        }

        // notify neighbors that they might need to re-mesh now that we have data
        for neighbor in chunk_manager.iter_neighbors(coord.pos) {
            if let ClientChunkState::NeedsMeshing { .. } | ClientChunkState::MeshComplete { .. } =
                neighbor.state
            {
                commands.entity(neighbor.entity).insert(CheckForMeshing);
            }
        }
    }
}

/// Determines chunks to promote to meshing based on the camera position and render distance.
#[instrument(skip_all)]
pub fn manage_distance_based_chunk_meshing_targets_system(
    // input
    anchor: StreamingAnchorQuery,

    // output
    mut chunk_manager: ResMut<ClientChunkManager>,
    mut commands: Commands,
) {
    let Some(camera_chunk_pos) = streaming_center(&anchor) else {
        return;
    };

    for (coord, state) in chunk_manager.chunk_states.iter_mut() {
        if is_within_horizontal_range(camera_chunk_pos, *coord, RENDER_DISTANCE)
            && let ClientChunkState::DataReady { entity } = state
        {
            debug!(target:"chunk_meshing", "Promoting chunk {:?} to WantsMeshing", coord);
            commands
                .entity(*entity)
                .insert((WantsMeshing, CheckForMeshing));
            *state = ClientChunkState::NeedsMeshing { entity: *entity };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Update;
    use common::world::chunk::ChunkLod;

    /// A chunk unloaded earlier in the same frame (its despawn still queued) must not be put
    /// back into the manager, or it sits in `NeedsMeshing` forever, is never requested again,
    /// and is despawned a second time on its next unload.
    #[test]
    fn chunk_unloaded_this_frame_is_not_promoted() {
        let mut app = App::new();
        app.init_resource::<ClientChunkManager>();
        app.add_systems(Update, promote_newly_generated_chunks_system);

        app.world_mut().spawn((
            crate::world::chunk::range::ChunkStreamingAnchor,
            ChunkCoord { pos: IVec3::ZERO },
        ));
        let tracked = IVec3::new(1, 0, 0);
        let unloaded = IVec3::new(2, 0, 0);
        let tracked_entity = app
            .world_mut()
            .spawn((
                ChunkCoord { pos: tracked },
                ChunkBlocksComponent::new_uniform_empty(ChunkLod(0)),
            ))
            .id();
        let unloaded_entity = app
            .world_mut()
            .spawn((
                ChunkCoord { pos: unloaded },
                ChunkBlocksComponent::new_uniform_empty(ChunkLod(0)),
            ))
            .id();
        // only `tracked` is still in the manager; `unloaded` was just dropped from it
        app.world_mut()
            .resource_mut::<ClientChunkManager>()
            .mark_as_data_ready(tracked, tracked_entity);

        app.update();

        let manager = app.world().resource::<ClientChunkManager>();
        assert_eq!(
            manager.get_state(tracked),
            Some(ClientChunkState::NeedsMeshing {
                entity: tracked_entity
            })
        );
        assert_eq!(manager.get_state(unloaded), None);
        assert!(
            !app.world()
                .entity(unloaded_entity)
                .contains::<WantsMeshing>()
        );
    }
}

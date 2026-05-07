use crate::player::{LocalPlayer, TargetedBlock};
use crate::prelude::*;
use crate::render::chunk::manager::ClientChunkManager;
use bevy::{
    ecs::relationship::Relationship,
    prelude::{Camera, Camera3d, ChildOf, Entity, GlobalTransform, Query, Res, ResMut, With},
};
use shared::{
    world::chunk::{ChunkCoord, chunk_blocks::ChunkView},
    world::{block::block_registry::AIR_BLOCK_ID, chunk::ChunkBlocksComponent},
};

/// Max raycast traverse distance in blocks
const RAYCAST_MAX_DIST: f32 = 8.0;
/// Traversal step size in blocks
const RAYCAST_STEP: f32 = 0.1;

/// Updates the `TargetedBlock` resource every frame based on the camera's raycast.
#[instrument(skip_all)]
pub fn update_targeted_block_system(
    // input
    camera_query: Query<(&GlobalTransform, &Camera, &ChildOf), With<Camera3d>>,
    player_query: Query<Entity, With<LocalPlayer>>,
    chunk_manager: Res<ClientChunkManager>,
    chunks_query: Query<&ChunkBlocksComponent>,

    // output
    mut targeted_block: ResMut<TargetedBlock>,
) {
    if player_query.is_empty() {
        return;
    }
    let local_player_entity = player_query.single().unwrap();

    let mut active_transform = None;
    for (transform, camera, parent) in camera_query.iter() {
        if camera.is_active && parent.get() == local_player_entity {
            active_transform = Some(transform);
            break;
        }
    }

    let Some(transform) = active_transform else {
        return;
    };

    // simple block raycast
    let mut last_block_pos = None;

    let mut target_pos = None;
    let mut target_normal = None;

    let steps = (RAYCAST_MAX_DIST / RAYCAST_STEP) as u32;

    for i in 0..steps {
        let dist = i as f32 * RAYCAST_STEP;
        let current_pos = transform.translation() + transform.forward() * dist;
        let current_block_pos = current_pos.floor().as_ivec3();

        // skip if we're still in the same block
        if Some(current_block_pos) == last_block_pos {
            continue;
        }

        // get block for current block
        let block_id = get_block_at_world_pos(current_block_pos, &chunk_manager, &chunks_query);

        // check if we hit something
        if block_id != Some(AIR_BLOCK_ID) && block_id.is_some() {
            target_pos = Some(current_block_pos);
            if let Some(last_pos) = last_block_pos {
                target_normal = Some(last_pos - current_block_pos);
            }
            break;
        }

        last_block_pos = Some(current_block_pos);
    }

    targeted_block.position = target_pos;
    targeted_block.normal = target_normal;
}

/// Helper function to get a block from world coordinates
fn get_block_at_world_pos(
    world_pos: IVec3,
    manager: &Res<ClientChunkManager>,
    chunks_query: &Query<&ChunkBlocksComponent>,
) -> Option<u8> {
    let (chunk_coord, local_pos) = ChunkCoord::world_to_chunk_and_local_pos(world_pos);

    // only get if chunk has data
    let state = manager.get_state(chunk_coord)?;
    if state.is_generated()
        && let Some(actual_entity) = state.entity()
        && let Ok(chunk_blocks) = chunks_query.get(actual_entity)
    {
        return Some(match chunk_blocks.get_view() {
            ChunkView::Uniform(block_id) => block_id,
            ChunkView::Dense(volume_view) => volume_view.get_data(
                local_pos.x as usize,
                local_pos.y as usize,
                local_pos.z as usize,
            ),
        });
    }

    None // chunk isn't loaded or doesn't have blocks
}

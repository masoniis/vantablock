use crate::player::TargetedBlock;
use crate::prelude::*;
use bevy::ecs::prelude::{Query, Res, ResMut, With};
use bevy::prelude::{Camera, Camera3d};
use bevy::transform::components::Transform;
use shared::simulation::chunk::{ChunkCoord, ChunkState, chunk_blocks::ChunkView};
use shared::simulation::{
    block::block_registry::AIR_BLOCK_ID,
    chunk::{ChunkBlocksComponent, ChunkStateManager},
};

/// Max raycast traverse distance in blocks
const RAYCAST_MAX_DIST: f32 = 8.0;
/// Traversal step size in blocks
const RAYCAST_STEP: f32 = 0.1;

/// Updates the `TargetedBlock` resource every frame based on the camera's raycast.
#[instrument(skip_all)]
pub fn update_targeted_block_system(
    // input
    camera_query: Query<(&Transform, &Camera), With<Camera3d>>,
    chunk_manager: Res<ChunkStateManager>,
    chunks_query: Query<&ChunkBlocksComponent>,

    // output
    mut targeted_block: ResMut<TargetedBlock>,
) {
    let mut active_transform = None;
    for (transform, camera) in camera_query.iter() {
        if camera.is_active {
            active_transform = Some(transform);
            break;
        }
    }

    let Some(transform) = active_transform else {
        return;
    };

    // simple voxel raycast
    let mut last_voxel_pos = None;

    let mut target_pos = None;
    let mut target_normal = None;

    let steps = (RAYCAST_MAX_DIST / RAYCAST_STEP) as u32;

    for i in 0..steps {
        let dist = i as f32 * RAYCAST_STEP;
        let current_pos = transform.translation + transform.forward() * dist;
        let current_voxel_pos = current_pos.floor().as_ivec3();

        // skip if we're still in the same voxel
        if Some(current_voxel_pos) == last_voxel_pos {
            continue;
        }

        // get block for current voxel
        let block_id = get_block_at_world_pos(current_voxel_pos, &chunk_manager, &chunks_query);

        // check if we hit something
        if block_id != Some(AIR_BLOCK_ID) && block_id.is_some() {
            target_pos = Some(current_voxel_pos);
            if let Some(last_pos) = last_voxel_pos {
                target_normal = Some(last_pos - current_voxel_pos);
            }
            break;
        }

        last_voxel_pos = Some(current_voxel_pos);
    }

    targeted_block.position = target_pos;
    targeted_block.normal = target_normal;
}

/// Helper function to get a block from world coordinates
fn get_block_at_world_pos(
    world_pos: IVec3,
    manager: &Res<ChunkStateManager>,
    chunks_query: &Query<&ChunkBlocksComponent>,
) -> Option<u8> {
    let (chunk_coord, local_pos) = ChunkCoord::world_to_chunk_and_local_pos(world_pos);

    // only get if chunk loaded
    let chunk_state = manager.get_state(chunk_coord)?;
    if let ChunkState::Loaded {
        entity: Some(actual_entity),
    } = chunk_state
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

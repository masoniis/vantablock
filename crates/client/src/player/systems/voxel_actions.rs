use crate::player::TargetedBlock;
use bevy::ecs::prelude::MessageWriter;
use bevy::ecs::prelude::Res;
use shared::simulation::player::actions::voxel::{BreakVoxelEvent, PlaceVoxelEvent};

/// Fires a `BreakVoxelEvent` for the currently targeted block.
pub fn break_targeted_voxel_system(
    targeted_block: Res<TargetedBlock>,
    mut break_voxel_writer: MessageWriter<BreakVoxelEvent>,
) {
    if let Some(voxel_pos) = targeted_block.position {
        break_voxel_writer.write(BreakVoxelEvent {
            world_pos: voxel_pos,
        });
    }
}

/// Fires a `PlaceVoxelEvent` for the currently targeted block.
pub fn place_targeted_voxel_system(
    targeted_block: Res<TargetedBlock>,
    mut place_voxel_writer: MessageWriter<PlaceVoxelEvent>,
) {
    if let (Some(voxel_pos), Some(normal)) = (targeted_block.position, targeted_block.normal) {
        place_voxel_writer.write(PlaceVoxelEvent {
            target_pos: voxel_pos + normal,
        });
    }
}

pub mod voxel;

pub use voxel::*;

// INFO: -------------------------------
//         actions module plugin
// -------------------------------------

use bevy::app::{App, Plugin, Update};
use bevy::ecs::message::Messages;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        // handle break voxel events
        app.init_resource::<Messages<voxel::break_targeted_voxel::BreakVoxelEvent>>()
            .add_systems(
                Update,
                voxel::break_targeted_voxel::handle_break_voxel_events_system,
            );

        // handle place voxel events
        app.init_resource::<Messages<voxel::place_voxel_at_target::PlaceVoxelEvent>>()
            .add_systems(
                Update,
                voxel::place_voxel_at_target::handle_place_voxel_events_system,
            );
    }
}

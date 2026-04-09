use crate::render::passes::{
    bounding_box::queue::BoundingBoxPhase, opaque::queue::Opaque3dRenderPhase,
    transparent::queue::Transparent3dRenderPhase,
};
use bevy::prelude::*;
use bevy::render::Extract;
use bevy::render::sync_world::RenderEntity;

/// System to extract active 3D cameras and initialize their custom render phases.
pub fn extract_custom_camera_phases_system(
    // in
    cameras_3d: Extract<Query<(&RenderEntity, &Camera), With<Camera3d>>>,
    //out
    mut commands: Commands,
) {
    for (render_entity, camera) in cameras_3d.iter() {
        if camera.is_active {
            commands.entity(render_entity.id()).insert((
                Opaque3dRenderPhase::default(),
                Transparent3dRenderPhase::default(),
                BoundingBoxPhase::default(),
            ));
        }
    }
}

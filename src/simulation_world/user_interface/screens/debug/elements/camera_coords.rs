use crate::prelude::*;
use crate::simulation_world::player::{ActiveCamera, CameraComponent};
use crate::simulation_world::{
    chunk::ChunkCoord, user_interface::components::UiText,
    user_interface::screens::debug_screen::CameraChunkCoordTextMarker,
};
use bevy::ecs::prelude::*;

/// Updates the content of the FPS counter text element.
#[instrument(skip_all)]
pub fn update_camera_chunk_coord_screen_text(
    // Input
    active_camera: Res<ActiveCamera>,
    camera_query: Query<(&CameraComponent, &ChunkCoord), Changed<ChunkCoord>>,

    // Output (updated component)
    mut query: Query<&mut UiText, With<CameraChunkCoordTextMarker>>,
) {
    if let Ok((_, chunk_chord)) = camera_query.get(active_camera.0) {
        if let Ok(mut ui_text) = query.single_mut() {
            ui_text.content = chunk_chord.to_string();
        } else {
            warn!("Failed to get single UiText with CameraChunkCoordTextMarker");
        }
    }
}

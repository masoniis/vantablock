use crate::input::systems::toggle_chunk_borders::ChunkBoundsToggle;
use bevy::ecs::resource::Resource;
use bevy::render::extract_resource::ExtractResource;

#[derive(Resource, Debug, PartialEq, Eq, Clone)]
pub struct WireframeToggleState {
    pub enabled: bool,
}

impl ExtractResource for WireframeToggleState {
    type Source = ChunkBoundsToggle;

    fn extract_resource(source: &Self::Source) -> Self {
        Self {
            enabled: source.enabled,
        }
    }
}

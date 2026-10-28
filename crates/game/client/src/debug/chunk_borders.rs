use crate::prelude::*;
use bevy::{
    ecs::{resource::Resource, system::ResMut},
    render::{RenderApp, extract_resource::ExtractResource},
};

#[derive(Resource, ExtractResource, Default, Clone)]
#[extract_app(RenderApp)]
pub struct ChunkBoundsToggle {
    pub enabled: bool,
}

/// Toggle chunk bounds mode on or off.
pub fn toggle_chunk_borders_system(mut mode: ResMut<ChunkBoundsToggle>) {
    info!("Toggling chunk borders: {}", !mode.enabled);
    mode.enabled = !mode.enabled;
}

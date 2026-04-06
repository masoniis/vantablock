use crate::prelude::*;
use bevy::ecs::{resource::Resource, system::ResMut};

#[derive(Resource, Default)]
pub struct OpaqueWireframeMode {
    pub enabled: bool,
}

/// Toggle opaque wireframe mode on or off.
pub fn toggle_opaque_wireframe_mode_system(mut mode: ResMut<OpaqueWireframeMode>) {
    info!("Toggling opaque wireframe mode: {}", !mode.enabled);
    mode.enabled = !mode.enabled;
}

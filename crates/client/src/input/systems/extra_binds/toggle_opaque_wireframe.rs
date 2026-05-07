use crate::prelude::*;
use bevy::{
    ecs::{resource::Resource, system::ResMut},
    render::extract_resource::ExtractResource,
};

/// A resource that defines the current opaque rendering polygon mode
#[derive(Resource, ExtractResource, Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum OpaqueRenderMode {
    #[default]
    Fill,
    Wireframe,
}

impl OpaqueRenderMode {
    pub fn next(self) -> Self {
        match self {
            Self::Fill => Self::Wireframe,
            Self::Wireframe => Self::Fill,
        }
    }
}

/// Toggle opaque wireframe mode on or off.
pub fn toggle_opaque_wireframe_mode_system(mut mode: ResMut<OpaqueRenderMode>) {
    *mode = mode.next();
    info!("Toggling opaque wireframe mode: {:?}", *mode);
}

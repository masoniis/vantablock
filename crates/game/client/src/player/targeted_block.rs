use crate::prelude::*;
use bevy::ecs::prelude::*;
use bevy::prelude::{Reflect, ReflectResource};
use bevy::render::{RenderApp, extract_resource::ExtractResource};

#[derive(Resource, ExtractResource, Clone, Default, Debug, Reflect)]
#[extract_app(RenderApp)]
#[reflect(Resource)]
pub struct TargetedBlock {
    pub position: Option<IVec3>,
    pub normal: Option<IVec3>,
}

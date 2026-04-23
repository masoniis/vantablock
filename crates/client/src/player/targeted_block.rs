use crate::prelude::*;
use bevy::ecs::prelude::*;
use bevy::render::extract_resource::ExtractResource;

#[derive(Resource, ExtractResource, Clone, Default, Debug)]
pub struct TargetedBlock {
    pub position: Option<IVec3>,
    pub normal: Option<IVec3>,
}

use crate::render::pipeline::VoxelMesh;
use bevy::asset::{AssetId};
use bevy::ecs::prelude::*;
use bevy::math::Vec3;
use shared::simulation::asset::VoxelMeshAsset;
use std::{collections::HashMap, sync::Arc};

#[derive(Resource, Default)]
pub struct RenderMeshStorageResource {
    pub meshes: HashMap<AssetId<VoxelMeshAsset>, Arc<VoxelMesh>>,
}

#[derive(Resource, Default)]
pub struct MeshesToUploadQueue {
    pub queue: Vec<(AssetId<VoxelMeshAsset>, VoxelMeshAsset, Vec3)>,
    pub removals: Vec<AssetId<VoxelMeshAsset>>,
}

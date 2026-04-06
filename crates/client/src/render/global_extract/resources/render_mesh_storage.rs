use crate::render::passes::VoxelMesh;
use bevy::asset::AssetId;
use bevy::ecs::prelude::*;
use bevy::math::Vec3;
use shared::simulation::asset_management::mesh_asset::VoxelChunkMeshAsset;
use std::{collections::HashMap, sync::Arc};

#[derive(Resource, Default)]
pub struct RenderMeshStorageResource {
    pub meshes: HashMap<AssetId<VoxelChunkMeshAsset>, Arc<VoxelMesh>>,
}

#[derive(Resource, Default)]
pub struct MeshesToUploadQueue {
    pub queue: Vec<(AssetId<VoxelChunkMeshAsset>, VoxelChunkMeshAsset, Vec3)>,
    pub removals: Vec<AssetId<VoxelChunkMeshAsset>>,
}

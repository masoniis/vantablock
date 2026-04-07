use crate::simulation::asset::VoxelMeshAsset;
use bevy::asset::Handle;
use bevy::ecs::prelude::Component;

#[derive(Component, Debug)]
pub struct OpaqueMeshComponent {
    pub mesh_handle: Handle<VoxelMeshAsset>,
}

impl OpaqueMeshComponent {
    /// Creates a new opaque-rendered mesh from raw vertex and index data.
    pub fn new(mesh_handle: Handle<VoxelMeshAsset>) -> Self {
        Self { mesh_handle }
    }
}

#[derive(Component, Debug)]
pub struct TransparentMeshComponent {
    pub mesh_handle: Handle<VoxelMeshAsset>,
}

impl TransparentMeshComponent {
    /// Creates a new transparent-rendered mesh from raw vertex and index data.
    pub fn new(mesh_handle: Handle<VoxelMeshAsset>) -> Self {
        Self { mesh_handle }
    }
}

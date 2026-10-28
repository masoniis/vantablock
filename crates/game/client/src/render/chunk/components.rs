use crate::render::chunk::asset::BlockMeshAsset;
use bevy::asset::Handle;
use bevy::ecs::prelude::Component;

#[derive(Component, Debug)]
pub struct OpaqueMeshComponent {
    pub mesh_handle: Handle<BlockMeshAsset>,
}

impl OpaqueMeshComponent {
    /// Creates a new opaque-rendered mesh from raw vertex and index data.
    pub fn new(mesh_handle: Handle<BlockMeshAsset>) -> Self {
        Self { mesh_handle }
    }
}

#[derive(Component, Debug)]
pub struct TransparentMeshComponent {
    pub mesh_handle: Handle<BlockMeshAsset>,
}

impl TransparentMeshComponent {
    /// Creates a new transparent-rendered mesh from raw vertex and index data.
    pub fn new(mesh_handle: Handle<BlockMeshAsset>) -> Self {
        Self { mesh_handle }
    }
}

/// Where a chunk is in the meshing lifecycle. The scheduler adds it (as `Queued`) when a loaded
/// chunk is within render distance and removes it if the chunk leaves while still queued; a
/// chunk without it is not wanted for meshing.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkMesh {
    /// Waiting for a task slot and for all 26 neighbours to have data.
    Queued,
    /// A meshing task is in flight.
    Meshing,
    /// Its current meshes are attached (or it has nothing visible).
    Meshed,
}

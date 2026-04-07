use crate::simulation::chunk::meshing::packed_face::PackedFace;
use bevy::asset::Asset;
use bevy::reflect::Reflect;
use std::sync::Arc;

// INFO: -----------------------------
//         types and resources
// -----------------------------------

/// A 3D voxel-based mesh asset consisting of packed face data.
#[derive(Asset, Reflect, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VoxelMeshAsset {
    pub name: String,
    pub faces: Arc<Vec<PackedFace>>,
}

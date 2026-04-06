use crate::simulation::chunk::meshing::packed_face::PackedFace;
use bevy::asset::Asset;
use bevy::reflect::Reflect;
use std::sync::Arc;

// INFO: -----------------------------
//         types and resources
// -----------------------------------

/// A 3D voxel-based mesh asset consisting of packed face data.
///
/// This is a custom asset that we now integrate with Bevy's native asset
/// system by deriving Asset and TypePath.
#[derive(Asset, Reflect, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VoxelChunkMeshAsset {
    pub name: String,
    pub faces: Arc<Vec<PackedFace>>,
}

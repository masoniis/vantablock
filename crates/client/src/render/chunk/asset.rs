use crate::render::chunk::meshing::packed_face::PackedFace;
use bevy::{asset::Asset, reflect::Reflect};
use std::sync::Arc;

// INFO: -----------------------------
//         types and resources
// -----------------------------------

/// A 3D block-based mesh asset consisting of packed face data.
#[derive(Asset, Reflect, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockMeshAsset {
    pub name: String,
    pub faces: Arc<[PackedFace]>,
}

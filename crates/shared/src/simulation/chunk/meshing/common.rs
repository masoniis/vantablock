use super::{OpaqueMeshData, TransparentMeshData};
use crate::simulation::chunk::meshing::packed_face::PackedFace;
use crate::simulation::{
    block::{BlockId, BlockRegistry},
    chunk::{NeighborLODs, PaddedChunk, types::ChunkLod},
};
use crate::{prelude::*, simulation::block::texture_registry::TextureId};
use std::sync::Arc;

// INFO: -----------------------
//         lookup tables
// -----------------------------

/// The 6 cardinal neighbor offsets.
///
/// Order: Top, Bottom, Left, Right, Front, Back
pub const NEIGHBOR_OFFSETS: [IVec3; 6] = [
    IVec3::new(0, 1, 0),  // top
    IVec3::new(0, -1, 0), // bottom
    IVec3::new(1, 0, 0),  // right
    IVec3::new(-1, 0, 0), // left
    IVec3::new(0, 0, 1),  // front
    IVec3::new(0, 0, -1), // back
];

/// Occlusion offsets blocks to check for each vertex.
#[rustfmt::skip]
pub const AO_OFFSETS: [[[IVec3; 3]; 4]; 6] = [
    [ // top face
        // front-left (starts at cube index 7 in shader)
        [IVec3::new(-1, 1, 0), IVec3::new(0, 1, 1), IVec3::new(-1, 1, 1)],
        // front-right
        [IVec3::new(1, 1, 0), IVec3::new(0, 1, 1), IVec3::new(1, 1, 1)],
        // back-right
        [IVec3::new(1, 1, 0), IVec3::new(0, 1, -1), IVec3::new(1, 1, -1)],
        // back-left
        [IVec3::new(-1, 1, 0), IVec3::new(0, 1, -1), IVec3::new(-1, 1, -1)],
    ],
    [ // bottom face
        // back-left (starts at cube vertex 0 in shader)
        [IVec3::new(-1, -1, 0), IVec3::new(0, -1, -1), IVec3::new(-1, -1, -1)],
        // back-right
        [IVec3::new(1, -1, 0), IVec3::new(0, -1, -1), IVec3::new(1, -1, -1)],
        // front-right
        [IVec3::new(1, -1, 0), IVec3::new(0, -1, 1), IVec3::new(1, -1, 1)],
        // front-left
        [IVec3::new(-1, -1, 0), IVec3::new(0, -1, 1), IVec3::new(-1, -1, 1)],
    ],
    [ // right face
        // bottom-left (starts at cube vertex 5 in shader)
        [IVec3::new(1, -1, 0), IVec3::new(1, 0, 1), IVec3::new(1, -1, 1)],
        // bottom-right 
        [IVec3::new(1, -1, 0), IVec3::new(1, 0, -1), IVec3::new(1, -1, -1)],
        // top-right
        [IVec3::new(1, 1, 0), IVec3::new(1, 0, -1), IVec3::new(1, 1, -1)],
        // top-left
        [IVec3::new(1, 1, 0), IVec3::new(1, 0, 1), IVec3::new(1, 1, 1)],
    ],
    [ // left face
        [IVec3::new(-1, -1, 0), IVec3::new(-1, 0, -1), IVec3::new(-1, -1, -1)],
        [IVec3::new(-1, -1, 0), IVec3::new(-1, 0, 1), IVec3::new(-1, -1, 1)],
        [IVec3::new(-1, 1, 0), IVec3::new(-1, 0, 1), IVec3::new(-1, 1, 1)],
        [IVec3::new(-1, 1, 0), IVec3::new(-1, 0, -1), IVec3::new(-1, 1, -1)],
    ],
    [ // front face
        [IVec3::new(-1, 0, 1), IVec3::new(0, -1, 1), IVec3::new(-1, -1, 1)],
        [IVec3::new(1, 0, 1), IVec3::new(0, -1, 1), IVec3::new(1, -1, 1)],
        [IVec3::new(1, 0, 1), IVec3::new(0, 1, 1), IVec3::new(1, 1, 1)],
        [IVec3::new(-1, 0, 1), IVec3::new(0, 1, 1), IVec3::new(-1, 1, 1)],
    ],
    [ // back face
        [IVec3::new(1, 0, -1), IVec3::new(0, -1, -1), IVec3::new(1, -1, -1)],
        [IVec3::new(-1, 0, -1), IVec3::new(0, -1, -1), IVec3::new(-1, -1, -1)],
        [IVec3::new(-1, 0, -1), IVec3::new(0, 1, -1), IVec3::new(-1, 1, -1)],
        [IVec3::new(1, 0, -1), IVec3::new(0, 1, -1), IVec3::new(1, 1, -1)],
    ],
];

// INFO: --------------------
//         util types
// --------------------------

/// Represents the 4 levels of Ambient Occlusion.
/// 0 = No Occlusion (Brightest), 3 = Max Occlusion (Darkest).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AoLevel {
    None = 0,   // 1.0 brightness
    Low = 1,    // 0.7 brightness
    Medium = 2, // 0.4 brightness
    High = 3,   // 0.2 brightness
}

impl From<u8> for AoLevel {
    #[inline(always)]
    fn from(v: u8) -> Self {
        match v {
            0 => AoLevel::None,
            1 => AoLevel::Low,
            2 => AoLevel::Medium,
            _ => AoLevel::High,
        }
    }
}

/// Represents the 6 cardinal directions (normals) for a voxel face.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceSide {
    Top = 0,    // +Y
    Bottom = 1, // -Y
    Right = 2,  // +X
    Left = 3,   // -X
    Front = 4,  // +Z
    Back = 5,   // -Z
}

impl FaceSide {
    /// All sides of a cube to be iterated over
    pub const ALL: [FaceSide; 6] = [
        FaceSide::Top,
        FaceSide::Bottom,
        FaceSide::Right,
        FaceSide::Left,
        FaceSide::Front,
        FaceSide::Back,
    ];

    #[inline]
    pub fn to_vec3(&self) -> [i32; 3] {
        match self {
            FaceSide::Top => [0, 1, 0],
            FaceSide::Bottom => [0, -1, 0],
            FaceSide::Right => [1, 0, 0],
            FaceSide::Left => [-1, 0, 0],
            FaceSide::Front => [0, 0, 1],
            FaceSide::Back => [0, 0, -1],
        }
    }
}

// INFO: ------------------------
//         util functions
// ------------------------------

/// Determine if a face should be rendered based on transparency rules
#[inline(always)]
pub fn should_render_face(
    current_id: BlockId,
    current_transparent: bool,
    neighbor_id: BlockId,
    neighbor_transparent: bool,
) -> bool {
    match (current_transparent, neighbor_transparent) {
        (false, true) => true,                     // opaque facing transparent
        (true, true) => current_id != neighbor_id, // different transparent blocks
        _ => false,
    }
}

/// Get the AO value (0-3) for a single vertex.
#[inline(always)]
pub fn get_ao(
    pos: IVec3,
    side1_offset: IVec3,
    side2_offset: IVec3,
    corner_offset: IVec3,
    padded_chunk: &PaddedChunk,
    transparency_lut: &[bool],
) -> u8 {
    let is_transparent =
        |id: BlockId| -> bool { unsafe { *transparency_lut.get_unchecked(id as usize) } };

    let s1_pos = pos + side1_offset;
    let s1 = !is_transparent(padded_chunk.get_block(s1_pos.x, s1_pos.y, s1_pos.z));
    let s2_pos = pos + side2_offset;
    let s2 = !is_transparent(padded_chunk.get_block(s2_pos.x, s2_pos.y, s2_pos.z));
    let c_pos = pos + corner_offset;
    let c = !is_transparent(padded_chunk.get_block(c_pos.x, c_pos.y, c_pos.z));

    if s1 && s2 {
        3
    } else {
        (s1 as u8) + (s2 as u8) + (c as u8)
    }
}

/// Calculates the ambient occlusion (ao) for a chunk position
pub fn calculate_ao_levels_for_face(
    pos: IVec3,
    face_side: FaceSide,
    padded_chunk: &PaddedChunk,
    transparency_lut: &[bool],
) -> [AoLevel; 4] {
    [
        get_ao(
            pos,
            AO_OFFSETS[face_side as usize][0][0],
            AO_OFFSETS[face_side as usize][0][1],
            AO_OFFSETS[face_side as usize][0][2],
            padded_chunk,
            transparency_lut,
        )
        .into(),
        get_ao(
            pos,
            AO_OFFSETS[face_side as usize][1][0],
            AO_OFFSETS[face_side as usize][1][1],
            AO_OFFSETS[face_side as usize][1][2],
            padded_chunk,
            transparency_lut,
        )
        .into(),
        get_ao(
            pos,
            AO_OFFSETS[face_side as usize][2][0],
            AO_OFFSETS[face_side as usize][2][1],
            AO_OFFSETS[face_side as usize][2][2],
            padded_chunk,
            transparency_lut,
        )
        .into(),
        get_ao(
            pos,
            AO_OFFSETS[face_side as usize][3][0],
            AO_OFFSETS[face_side as usize][3][1],
            AO_OFFSETS[face_side as usize][3][2],
            padded_chunk,
            transparency_lut,
        )
        .into(),
    ]
}

/// Builds the mesh assets given chunk vertices
#[instrument(skip_all)]
pub fn build_mesh_assets(
    name: &str,
    opaque_faces: Vec<PackedFace>,
    transparent_faces: Vec<PackedFace>,
) -> (Option<OpaqueMeshData>, Option<TransparentMeshData>) {
    let opaque = if !opaque_faces.is_empty() {
        Some(OpaqueMeshData {
            name: name.to_string(),
            faces: Arc::new(opaque_faces),
        })
    } else {
        None
    };
    let trans = if !transparent_faces.is_empty() {
        Some(TransparentMeshData {
            name: format!("{}_trans", name),
            faces: Arc::new(transparent_faces),
        })
    } else {
        None
    };
    (opaque, trans)
}

pub struct MesherContext<'a, R> {
    pub padded_chunk: &'a PaddedChunk,
    pub block_registry: &'a BlockRegistry,
    pub render_registry: &'a R,
    pub center_lod: ChunkLod,
    pub neighbor_lods: &'a NeighborLODs,
    pub chunk_size: usize,
    pub scale: f32,
}

impl<'a, R> MesherContext<'a, R> {
    #[inline(always)]
    pub fn push_face(
        &self,
        face_side: FaceSide,
        block_pos: IVec3,
        tex_id: TextureId,
        ao_levels: [AoLevel; 4],
        out_faces: &mut Vec<PackedFace>,
    ) {
        let face = PackedFace::new(
            block_pos.x as u32,
            block_pos.y as u32,
            block_pos.z as u32,
            face_side,
            ao_levels,
            tex_id,
        );

        out_faces.push(face);
    }
}

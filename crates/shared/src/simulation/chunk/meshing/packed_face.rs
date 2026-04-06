use crate::simulation::chunk::meshing::common::{AoLevel, FaceSide};
use bevy::reflect::Reflect;

/// A struct representing a single voxel face in the world
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    bytemuck::Pod,
    bytemuck::Zeroable,
    Hash,
    Reflect,
)]
#[reflect(opaque)]
pub struct PackedFace(pub u32);

impl PackedFace {
    // mask sizes for each category
    const POS_MASK: u32 = 0b11111; // 5 bits (0-31)
    const AO_MASK: u32 = 0b11; // 2 bits (0-3)
    const NORM_MASK: u32 = 0b111; // 3 bits (0-7)
    const TEX_MASK: u32 = 0b111111; // 6 bits (0-63)

    // shift distances
    const Y_SHIFT: u32 = 5;
    const Z_SHIFT: u32 = 10;
    const AO_SHIFT: u32 = 15;
    const NORM_SHIFT: u32 = 23;
    const TEX_SHIFT: u32 = 26;

    /// Packs face data into a densely packed `PackedFace`.
    #[inline(always)]
    pub fn new(
        x: u32,
        y: u32,
        z: u32,
        normal: FaceSide,
        ao_corners: [AoLevel; 4],
        texture_id: u32,
    ) -> Self {
        let mut packed = 0u32;

        // 15 position bits
        packed |= x & Self::POS_MASK;
        packed |= (y & Self::POS_MASK) << Self::Y_SHIFT;
        packed |= (z & Self::POS_MASK) << Self::Z_SHIFT;

        // 8 ambient occlusion bits (2 for each corner)
        packed |= (ao_corners[0] as u32 & Self::AO_MASK) << Self::AO_SHIFT;
        packed |= (ao_corners[1] as u32 & Self::AO_MASK) << (Self::AO_SHIFT + 2);
        packed |= (ao_corners[2] as u32 & Self::AO_MASK) << (Self::AO_SHIFT + 4);
        packed |= (ao_corners[3] as u32 & Self::AO_MASK) << (Self::AO_SHIFT + 6);

        // 3 normal bits
        packed |= (normal as u32 & Self::NORM_MASK) << Self::NORM_SHIFT;

        // 6 tid bits 😳
        packed |= (texture_id & Self::TEX_MASK) << Self::TEX_SHIFT;

        PackedFace(packed)
    }
}

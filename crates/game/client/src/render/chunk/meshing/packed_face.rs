use crate::render::chunk::meshing::common::{AoLevel, FaceSide};
use bevy::reflect::Reflect;

/// A struct representing a single block face in the world
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

    /// The number of distinct texture ids a face can address.
    pub const MAX_TEXTURES: usize = Self::TEX_MASK as usize + 1;

    /// Packs face data into a densely packed `PackedFace`.
    ///
    /// Out-of-range inputs are masked (and so wrap) in release builds; debug builds assert.
    #[inline(always)]
    pub fn new(
        x: u32,
        y: u32,
        z: u32,
        normal: FaceSide,
        ao_corners: [AoLevel; 4],
        texture_id: u32,
    ) -> Self {
        debug_assert!(
            x <= Self::POS_MASK && y <= Self::POS_MASK && z <= Self::POS_MASK,
            "face position ({x}, {y}, {z}) does not fit in 5 bits"
        );
        debug_assert!(
            texture_id <= Self::TEX_MASK,
            "texture id {texture_id} does not fit in 6 bits"
        );

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the layout the WESL unpacker in `face_unpacking.wesl` relies on.
    #[test]
    fn fields_land_at_the_documented_bit_offsets() {
        let ao = [AoLevel::Low, AoLevel::Medium, AoLevel::High, AoLevel::None];
        let face = PackedFace::new(1, 2, 3, FaceSide::Left, ao, 42);

        assert_eq!(face.0 & 0x1F, 1);
        assert_eq!((face.0 >> 5) & 0x1F, 2);
        assert_eq!((face.0 >> 10) & 0x1F, 3);
        assert_eq!((face.0 >> 15) & 0x3, 1);
        assert_eq!((face.0 >> 17) & 0x3, 2);
        assert_eq!((face.0 >> 19) & 0x3, 3);
        assert_eq!((face.0 >> 21) & 0x3, 0);
        assert_eq!((face.0 >> 23) & 0x7, FaceSide::Left as u32);
        assert_eq!(face.0 >> 26, 42);
    }

    #[test]
    // the digit groups follow the field widths: texture, normal, AO, z, y, x
    #[allow(clippy::unusual_byte_groupings)]
    fn max_values_fill_the_word_without_overlap() {
        let face = PackedFace::new(31, 31, 31, FaceSide::Back, [AoLevel::High; 4], 63);
        assert_eq!(face.0, 0b111111_101_11111111_11111_11111_11111);
    }
}

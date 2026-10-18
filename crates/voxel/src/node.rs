//! The 16-byte `Node64` topology header shared bit-for-bit with WGSL.
//!
//! ```wgsl
//! struct Node64 {
//!     child_mask: vec2<u32>, // 8 B: 64 occupancy bits
//!     child_ref:  u32,       // 4 B: full logical arena handle, typed by context
//!     lod_meta:   u32,       // 4 B: bit31 leaf tag, low31 LOD index
//! };
//! ```
//!
//! An internal node's `child_ref` points at the first of its contiguous occupied-child headers
//! in the topology arena. A terminal node's `child_ref` points at a contiguous block of leaf
//! attribute words, one per occupied voxel. The two cases are distinguished by the leaf tag in
//! `lod_meta`, never by bits stolen from `child_ref`.

use crate::{handle::ArenaHandle, mask::Mask64};
use bytemuck::{Pod, Zeroable};

/// Size of a `Node64` in 32-bit words, the stride used when walking a child block.
pub const NODE_WORDS: u32 = 4;
/// Bit 31 of `lod_meta`: set on terminal nodes whose children are voxels.
pub const LEAF_TAG: u32 = 0x8000_0000;
/// Low 31 bits of `lod_meta`: index of the node's LOD summary record.
pub const LOD_INDEX_MASK: u32 = 0x7FFF_FFFF;
/// Reserved LOD index meaning "no summary baked for this node".
pub const LOD_MISSING: u32 = LOD_INDEX_MASK;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct Node64 {
    pub child_mask: Mask64,
    pub child_ref: u32,
    pub lod_meta: u32,
}

const _: () = assert!(std::mem::size_of::<Node64>() == 16);

impl Node64 {
    /// A node whose children are `Node64` headers starting at `children`.
    pub fn internal(mask: Mask64, children: ArenaHandle, lod_index: Option<u32>) -> Self {
        Self {
            child_mask: mask,
            child_ref: children.raw(),
            lod_meta: encode_lod(lod_index),
        }
    }

    /// A node whose children are voxels with attributes starting at `attributes`.
    pub fn terminal(mask: Mask64, attributes: ArenaHandle, lod_index: Option<u32>) -> Self {
        Self {
            child_mask: mask,
            child_ref: attributes.raw(),
            lod_meta: LEAF_TAG | encode_lod(lod_index),
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.lod_meta & LEAF_TAG != 0
    }

    pub fn child_ref(&self) -> ArenaHandle {
        ArenaHandle::from_raw(self.child_ref)
    }

    pub fn lod_index(&self) -> Option<u32> {
        let index = self.lod_meta & LOD_INDEX_MASK;
        (index != LOD_MISSING).then_some(index)
    }

    pub fn to_words(self) -> [u32; 4] {
        bytemuck::cast(self)
    }

    pub fn from_words(words: [u32; 4]) -> Self {
        bytemuck::cast(words)
    }
}

fn encode_lod(lod_index: Option<u32>) -> u32 {
    match lod_index {
        Some(index) => {
            assert!(
                index < LOD_MISSING,
                "LOD index {index} exceeds the 31-bit format"
            );
            index
        }
        None => LOD_MISSING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_layout_matches_wgsl() {
        let node = Node64::internal(
            Mask64 {
                lo: 0xAAAA_0001,
                hi: 0x8000_0000,
            },
            ArenaHandle::new(2, 40).unwrap(),
            Some(9),
        );
        let words = node.to_words();
        assert_eq!(words[0], 0xAAAA_0001);
        assert_eq!(words[1], 0x8000_0000);
        assert_eq!(words[2], ArenaHandle::new(2, 40).unwrap().raw());
        assert_eq!(words[3], 9);
        assert_eq!(Node64::from_words(words), node);
    }

    #[test]
    fn leaf_tag_does_not_touch_the_handle() {
        // a handle using every bit must survive next to a terminal tag
        let handle = ArenaHandle::new(254, (1 << 24) - 1).unwrap();
        let leaf = Node64::terminal(Mask64::FULL, handle, None);
        assert!(leaf.is_terminal());
        assert_eq!(leaf.child_ref(), handle);
        assert_eq!(leaf.lod_index(), None);

        let internal = Node64::internal(Mask64::FULL, handle, Some(LOD_MISSING - 1));
        assert!(!internal.is_terminal());
        assert_eq!(internal.lod_index(), Some(LOD_MISSING - 1));
    }

    #[test]
    #[should_panic]
    fn lod_index_capacity_is_enforced() {
        let _ = Node64::internal(Mask64::FULL, ArenaHandle::INVALID, Some(LOD_MISSING));
    }
}

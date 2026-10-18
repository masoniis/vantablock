//! Portable 64-bit occupancy masks stored as two `u32` halves.
//!
//! WGSL has no portable `u64`, so every node's 64 child bits live in a `vec2<u32>`. The prefix
//! count here is a line-for-line mirror of `child_word_offset` in
//! `assets/client/shader/voxel/lib/mask.wesl`; keep the two in sync.

use crate::node::NODE_WORDS;
use bytemuck::{Pod, Zeroable};

/// Child index `k` of the cell at `(x, y, z)` inside a `4 x 4 x 4` node.
#[inline]
pub const fn cell_index(x: u32, y: u32, z: u32) -> u32 {
    x + 4 * y + 16 * z
}

/// The `(x, y, z)` cell of child index `k`, the inverse of [`cell_index`].
#[inline]
pub const fn cell_coords(k: u32) -> (u32, u32, u32) {
    (k & 3, (k >> 2) & 3, k >> 4)
}

/// 64 occupancy bits, one per cell of a `4 x 4 x 4` node, indexed by [`cell_index`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct Mask64 {
    /// Bits for children `0..32`.
    pub lo: u32,
    /// Bits for children `32..64`.
    pub hi: u32,
}

impl Mask64 {
    pub const EMPTY: Self = Self { lo: 0, hi: 0 };
    pub const FULL: Self = Self {
        lo: u32::MAX,
        hi: u32::MAX,
    };

    pub const fn from_u64(bits: u64) -> Self {
        Self {
            lo: bits as u32,
            hi: (bits >> 32) as u32,
        }
    }

    pub const fn to_u64(self) -> u64 {
        self.lo as u64 | ((self.hi as u64) << 32)
    }

    pub const fn is_empty(self) -> bool {
        self.lo == 0 && self.hi == 0
    }

    pub const fn is_set(self, k: u32) -> bool {
        if k < 32 {
            self.lo & (1 << k) != 0
        } else {
            self.hi & (1 << (k - 32)) != 0
        }
    }

    pub fn set(&mut self, k: u32) {
        if k < 32 {
            self.lo |= 1 << k;
        } else {
            self.hi |= 1 << (k - 32);
        }
    }

    pub const fn count(self) -> u32 {
        self.lo.count_ones() + self.hi.count_ones()
    }

    /// Iterates the set child indices in ascending order, which is also the order children are
    /// laid out in their contiguous block.
    pub fn iter(self) -> impl Iterator<Item = u32> {
        let mut bits = self.to_u64();
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let k = bits.trailing_zeros();
            bits &= bits - 1;
            Some(k)
        })
    }

    /// Number of occupied children strictly below `k` (`k` in `0..64`, child `k` excluded).
    ///
    /// This is the position of child `k` inside its parent's contiguous child block. Mirrors the
    /// branchless WGSL baseline exactly.
    #[inline]
    pub fn prefix_count(self, k: u32) -> u32 {
        let high = k >= 32;
        let below = (1u32 << (k & 31)) - 1;
        let lower = if high { self.lo } else { self.lo & below }.count_ones();
        let upper = if high { self.hi & below } else { 0 }.count_ones();
        lower + upper
    }

    /// The 32-bit word offset of child `k`'s `Node64` header within its contiguous block.
    ///
    /// Offsets are within one block only; they never cross a page (see [`crate::arena`]).
    #[inline]
    pub fn child_word_offset(self, k: u32) -> u32 {
        self.prefix_count(k) * NODE_WORDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference: a plain `u64` popcount below `k`.
    fn reference_prefix(bits: u64, k: u32) -> u32 {
        (bits & ((1u64 << k) - 1)).count_ones()
    }

    /// Deterministic masks without pulling in an rng dependency.
    fn splitmix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn check(bits: u64) {
        let mask = Mask64::from_u64(bits);
        for k in 0..64 {
            let expected = reference_prefix(bits, k);
            assert_eq!(
                mask.prefix_count(k),
                expected,
                "mask {bits:#018x} child {k}"
            );
            assert_eq!(mask.child_word_offset(k), expected * NODE_WORDS);
        }
    }

    #[test]
    fn empty_and_full_masks() {
        check(0);
        check(u64::MAX);
        for k in 0..64 {
            assert_eq!(Mask64::FULL.prefix_count(k), k);
            assert_eq!(Mask64::EMPTY.prefix_count(k), 0);
        }
    }

    #[test]
    fn half_boundaries() {
        // the lo/hi split is the most likely place for an off-by-one
        for bits in [
            1u64 << 31,
            1u64 << 32,
            (1u64 << 31) | (1u64 << 32),
            0xFFFF_FFFF,
            0xFFFF_FFFF_0000_0000,
        ] {
            check(bits);
        }
        let straddle = Mask64::from_u64((1u64 << 31) | (1u64 << 32));
        assert_eq!(straddle.prefix_count(31), 0);
        assert_eq!(straddle.prefix_count(32), 1);
        assert_eq!(straddle.prefix_count(33), 2);
    }

    #[test]
    fn isolated_bits() {
        for bit in 0..64 {
            check(1u64 << bit);
            check(!(1u64 << bit));
        }
    }

    #[test]
    fn randomized_masks() {
        let mut state = 0x5EED;
        for _ in 0..4096 {
            check(splitmix(&mut state));
        }
        // sparse masks resemble real foliage nodes more than uniform random bits do
        for _ in 0..1024 {
            check(splitmix(&mut state) & splitmix(&mut state) & splitmix(&mut state));
        }
    }

    #[test]
    fn set_iter_and_count_agree() {
        let mut state = 7;
        for _ in 0..256 {
            let bits = splitmix(&mut state);
            let mask = Mask64::from_u64(bits);
            let mut rebuilt = Mask64::EMPTY;
            for (i, k) in mask.iter().enumerate() {
                assert!(mask.is_set(k));
                assert_eq!(mask.prefix_count(k), i as u32);
                rebuilt.set(k);
            }
            assert_eq!(rebuilt, mask);
            assert_eq!(mask.count(), bits.count_ones());
        }
    }

    #[test]
    fn cell_index_round_trips() {
        for k in 0..64 {
            let (x, y, z) = cell_coords(k);
            assert_eq!(cell_index(x, y, z), k);
        }
    }
}

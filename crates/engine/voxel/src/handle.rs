//! Logical 32-bit arena handles: `8-bit logical page ID + 24-bit word offset`.
//!
//! A handle names a 32-bit word inside one logical page of an arena (at most 64 MiB per page).
//! It is a *logical* address: the GPU resolves a page to a resident slab location through a page
//! table (see [`crate::residency`]). The handle format is provisional and benchmark-gated.
//!
//! Which arena a handle points into is decided by the consuming operation (e.g. a terminal
//! node's `child_ref` is an attribute handle), never by spare bits in the handle itself. Every bit
//! is spent on the address; the leaf tag lives in [`crate::node::Node64::lod_meta`].

use bytemuck::{Pod, Zeroable};

pub const PAGE_ID_BITS: u32 = 8;
pub const WORD_OFFSET_BITS: u32 = 24;
/// Words addressable within a single page (`2^24` words = 64 MiB).
pub const MAX_PAGE_WORDS: u32 = 1 << WORD_OFFSET_BITS;
const WORD_OFFSET_MASK: u32 = MAX_PAGE_WORDS - 1;
/// Page IDs are `0..MAX_PAGES`. Page `0xFF` is reserved so the all-ones sentinel never aliases
/// a real word.
pub const MAX_PAGES: u32 = (1 << PAGE_ID_BITS) - 1;

/// An untyped logical arena address. See the [module docs](self).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Pod, Zeroable)]
pub struct ArenaHandle(u32);

impl ArenaHandle {
    /// The reserved "no data" sentinel.
    pub const INVALID: Self = Self(u32::MAX);

    /// Builds a handle, returning `None` when the page or offset is out of range.
    pub const fn new(page: u32, word_offset: u32) -> Option<Self> {
        if page >= MAX_PAGES || word_offset >= MAX_PAGE_WORDS {
            return None;
        }
        Some(Self((page << WORD_OFFSET_BITS) | word_offset))
    }

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn page(self) -> u32 {
        self.0 >> WORD_OFFSET_BITS
    }

    pub const fn word_offset(self) -> u32 {
        self.0 & WORD_OFFSET_MASK
    }

    pub const fn is_valid(self) -> bool {
        self.0 != Self::INVALID.0
    }

    /// The handle `words` further into the same page.
    ///
    /// Callers only offset within a single contiguous block, and blocks never straddle pages, so
    /// overflowing the page is a logic error.
    pub fn offset(self, words: u32) -> Self {
        let offset = self.word_offset() + words;
        debug_assert!(
            offset < MAX_PAGE_WORDS,
            "handle offset crossed a page boundary"
        );
        Self((self.page() << WORD_OFFSET_BITS) | offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trips() {
        for page in [0, 1, 17, MAX_PAGES - 1] {
            for offset in [0, 1, 4095, MAX_PAGE_WORDS - 1] {
                let h = ArenaHandle::new(page, offset).unwrap();
                assert_eq!(h.page(), page);
                assert_eq!(h.word_offset(), offset);
                assert!(h.is_valid());
            }
        }
    }

    #[test]
    fn out_of_range_and_sentinel_are_rejected() {
        assert!(ArenaHandle::new(MAX_PAGES, 0).is_none());
        assert!(ArenaHandle::new(0, MAX_PAGE_WORDS).is_none());
        // the highest legal handle is still distinct from the sentinel
        let last = ArenaHandle::new(MAX_PAGES - 1, MAX_PAGE_WORDS - 1).unwrap();
        assert_ne!(last, ArenaHandle::INVALID);
        assert!(!ArenaHandle::INVALID.is_valid());
    }

    #[test]
    fn offsets_stay_in_page() {
        let h = ArenaHandle::new(3, 100).unwrap();
        let next = h.offset(4);
        assert_eq!(next.page(), 3);
        assert_eq!(next.word_offset(), 104);
    }
}

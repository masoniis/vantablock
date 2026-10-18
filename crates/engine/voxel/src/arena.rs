//! Paged word arenas for topology and leaf attributes.
//!
//! Each arena is a list of logical pages of 32-bit words. Allocation is block-based, and the
//! one hard rule is that **no block crosses a page boundary**: a contiguous child block or leaf
//! attribute block always lives inside a single page, so `base + popcount * stride` can never wrap
//! into a different page. A logical asset may still span many pages.
//!
//! Arenas are typed by what consumes them ([`Topology`], [`Attributes`]) so a
//! handle into one can't silently be used to read another.

use crate::handle::{ArenaHandle, MAX_PAGE_WORDS, MAX_PAGES};
use std::{collections::HashMap, fmt, hash::Hash, marker::PhantomData};

/// Marks which kind of data an arena holds.
pub trait ArenaKind: 'static {
    const NAME: &'static str;
    /// Words per element, the stride used when indexing into one of this arena's blocks.
    const STRIDE_WORDS: u32;
}

/// Hot: `Node64` headers walked during hierarchical traversal.
#[derive(Clone, Copy, Debug)]
pub struct Topology;
impl ArenaKind for Topology {
    const NAME: &'static str = "topology";
    const STRIDE_WORDS: u32 = crate::node::NODE_WORDS;
}

/// Cold: one word per occupied voxel, fetched only after an actual leaf hit.
#[derive(Clone, Copy, Debug)]
pub struct Attributes;
impl ArenaKind for Attributes {
    const NAME: &'static str = "attributes";
    const STRIDE_WORDS: u32 = 1;
}

/// A handle that can only be used with arenas of kind `K`.
pub struct Handle<K> {
    raw: ArenaHandle,
    _kind: PhantomData<K>,
}

impl<K> Handle<K> {
    pub const fn from_untyped(raw: ArenaHandle) -> Self {
        Self {
            raw,
            _kind: PhantomData,
        }
    }

    pub const fn untyped(self) -> ArenaHandle {
        self.raw
    }
}

impl<K> Clone for Handle<K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K> Copy for Handle<K> {}
impl<K> PartialEq for Handle<K> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<K> Eq for Handle<K> {}
impl<K> Hash for Handle<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
impl<K: ArenaKind> fmt::Debug for Handle<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Handle<{}>(page {}, word {})",
            K::NAME,
            self.raw.page(),
            self.raw.word_offset()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArenaError {
    /// The block is larger than a single page and so can never be placed.
    BlockTooLarge {
        arena: &'static str,
        words: usize,
        page_words: u32,
    },
    /// Every logical page ID is in use.
    OutOfPages { arena: &'static str },
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlockTooLarge {
                arena,
                words,
                page_words,
            } => write!(
                f,
                "{arena} block of {words} words exceeds the {page_words}-word page size"
            ),
            Self::OutOfPages { arena } => write!(f, "{arena} arena ran out of logical pages"),
        }
    }
}

impl std::error::Error for ArenaError {}

/// Allocation counters, reported by diagnostics.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArenaStats {
    pub blocks: u64,
    pub words: u64,
    /// Blocks that were satisfied by an existing identical block.
    pub dedup_hits: u64,
    pub dedup_words_saved: u64,
}

/// A paged arena of 32-bit words. See the [module docs](self).
pub struct WordArena<K: ArenaKind> {
    page_words: u32,
    pages: Vec<Vec<u32>>,
    /// Identical blocks share storage. Keys hold the full block, so a hash match is always
    /// confirmed by full equality before reuse.
    dedup: HashMap<Box<[u32]>, Handle<K>>,
    stats: ArenaStats,
}

impl<K: ArenaKind> Default for WordArena<K> {
    fn default() -> Self {
        Self::new(MAX_PAGE_WORDS)
    }
}

impl<K: ArenaKind> WordArena<K> {
    /// Creates an arena with pages of `page_words` words. Production uses the full 24-bit
    /// page; tests use tiny pages to exercise page-boundary behaviour.
    pub fn new(page_words: u32) -> Self {
        assert!(
            (1..=MAX_PAGE_WORDS).contains(&page_words),
            "page size must be in 1..=2^24 words"
        );
        Self {
            page_words,
            pages: Vec::new(),
            dedup: HashMap::new(),
            stats: ArenaStats::default(),
        }
    }

    pub fn page_words(&self) -> u32 {
        self.page_words
    }

    pub fn pages(&self) -> &[Vec<u32>] {
        &self.pages
    }

    pub fn stats(&self) -> ArenaStats {
        self.stats
    }

    pub fn used_words(&self) -> u64 {
        self.pages.iter().map(|p| p.len() as u64).sum()
    }

    /// Words still free in the current (last) page.
    pub fn current_page_remaining(&self) -> u32 {
        self.pages
            .last()
            .map_or(0, |p| self.page_words - p.len() as u32)
    }

    /// The page the next allocation of `words` words would land in.
    pub fn next_page_for(&self, words: u32) -> u32 {
        if self.pages.is_empty() || self.current_page_remaining() < words {
            self.pages.len() as u32
        } else {
            self.pages.len() as u32 - 1
        }
    }

    /// Opens a fresh page unless the current page already has `words` free, so the next
    /// `words` words of allocations are guaranteed to share a page.
    pub fn reserve_contiguous(&mut self, words: u32) -> Result<(), ArenaError> {
        if words > self.page_words || self.current_page_remaining() >= words {
            return Ok(());
        }
        self.open_page()
    }

    /// Appends a block, keeping it inside a single page.
    pub fn alloc(&mut self, block: &[u32]) -> Result<Handle<K>, ArenaError> {
        if block.len() > self.page_words as usize {
            return Err(ArenaError::BlockTooLarge {
                arena: K::NAME,
                words: block.len(),
                page_words: self.page_words,
            });
        }
        if self.pages.is_empty() || (self.current_page_remaining() as usize) < block.len() {
            self.open_page()?;
        }

        let page_id = self.pages.len() as u32 - 1;
        let page = self.pages.last_mut().expect("a page was just ensured");
        let offset = page.len() as u32;
        page.extend_from_slice(block);

        self.stats.blocks += 1;
        self.stats.words += block.len() as u64;
        Ok(Handle::from_untyped(
            ArenaHandle::new(page_id, offset).expect("page and offset are in range"),
        ))
    }

    /// Like [`alloc`](Self::alloc), but reuses an identical existing block.
    pub fn alloc_dedup(&mut self, block: &[u32]) -> Result<Handle<K>, ArenaError> {
        if let Some(&existing) = self.dedup.get(block) {
            self.stats.dedup_hits += 1;
            self.stats.dedup_words_saved += block.len() as u64;
            return Ok(existing);
        }
        let handle = self.alloc(block)?;
        self.dedup.insert(block.into(), handle);
        Ok(handle)
    }

    /// Drops the deduplication index (e.g. once baking is finished) to free its memory.
    pub fn finish_dedup(&mut self) {
        self.dedup = HashMap::new();
    }

    pub fn word(&self, handle: Handle<K>) -> u32 {
        let h = handle.untyped();
        self.pages[h.page() as usize][h.word_offset() as usize]
    }

    pub fn block(&self, handle: Handle<K>, words: u32) -> &[u32] {
        let h = handle.untyped();
        let start = h.word_offset() as usize;
        &self.pages[h.page() as usize][start..start + words as usize]
    }

    fn open_page(&mut self) -> Result<(), ArenaError> {
        if self.pages.len() as u32 >= MAX_PAGES {
            return Err(ArenaError::OutOfPages { arena: K::NAME });
        }
        self.pages.push(Vec::new());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_never_straddle_pages() {
        let mut arena = WordArena::<Topology>::new(10);
        let a = arena.alloc(&[1; 8]).unwrap();
        // 2 words remain in page 0, so a 4 word block must start page 1
        let b = arena.alloc(&[2; 4]).unwrap();
        let c = arena.alloc(&[3; 6]).unwrap();
        assert_eq!((a.untyped().page(), a.untyped().word_offset()), (0, 0));
        assert_eq!((b.untyped().page(), b.untyped().word_offset()), (1, 0));
        assert_eq!((c.untyped().page(), c.untyped().word_offset()), (1, 4));
        assert_eq!(arena.block(c, 6), &[3; 6]);

        // a block that exactly fills the remaining page stays put
        let mut exact = WordArena::<Topology>::new(8);
        exact.alloc(&[0; 4]).unwrap();
        let fill = exact.alloc(&[1; 4]).unwrap();
        assert_eq!(fill.untyped().page(), 0);
        assert_eq!(exact.pages().len(), 1);
    }

    #[test]
    fn oversized_blocks_are_rejected() {
        let mut arena = WordArena::<Attributes>::new(4);
        assert!(matches!(
            arena.alloc(&[0; 5]),
            Err(ArenaError::BlockTooLarge { .. })
        ));
    }

    #[test]
    fn page_ids_are_bounded() {
        let mut arena = WordArena::<Attributes>::new(1);
        for _ in 0..MAX_PAGES {
            arena.alloc(&[0]).unwrap();
        }
        assert_eq!(
            arena.alloc(&[0]),
            Err(ArenaError::OutOfPages {
                arena: "attributes"
            })
        );
    }

    #[test]
    fn dedup_requires_full_equality() {
        let mut arena = WordArena::<Attributes>::default();
        let a = arena.alloc_dedup(&[1, 2, 3]).unwrap();
        let b = arena.alloc_dedup(&[1, 2, 3]).unwrap();
        let c = arena.alloc_dedup(&[1, 2, 4]).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(arena.stats().dedup_hits, 1);
        assert_eq!(arena.used_words(), 6);
    }

    #[test]
    fn reserve_contiguous_opens_a_page_only_when_needed() {
        let mut arena = WordArena::<Topology>::new(16);
        arena.alloc(&[0; 10]).unwrap();
        arena.reserve_contiguous(4).unwrap();
        assert_eq!(arena.pages().len(), 1);
        arena.reserve_contiguous(8).unwrap();
        assert_eq!(arena.pages().len(), 2);
        assert_eq!(arena.next_page_for(8), 1);
    }
}

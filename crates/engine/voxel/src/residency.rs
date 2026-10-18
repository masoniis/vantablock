//! Mapping logical arena pages onto resident GPU storage.
//!
//! Pages are packed whole, back to back, into at most [`MAX_SLABS`] slabs per arena, each one
//! storage binding no larger than the limits the device was *actually created with*. A page
//! never straddles slabs, so a child block (which never straddles a page) is always read from a
//! single binding.
//!
//! The page table maps each logical page to a slab address: the slab index in the top
//! [`SLAB_BITS`] bits and the page's first word within that slab below them. A handle resolves to
//! `page_entry[page] + word_offset`, still a slab address, and the shader reads it through a
//! small switch whose first case is slab 0. When everything fits one slab (the common case) that
//! branch is uniform, and a voxel root whose topology lives in one page resolves its entry once
//! at root entry instead of per node.
//!
//! Geometry that needs more slabs than there are bindings is reported precisely, never silently
//! treated as empty.

use std::fmt;

/// Slab bindings per arena the shaders declare.
pub const MAX_SLABS: usize = 4;
/// Bits of a slab address naming the slab.
pub const SLAB_BITS: u32 = 2;
pub const SLAB_SHIFT: u32 = 32 - SLAB_BITS;
/// Bits of a slab address naming the word within its slab.
pub const SLAB_WORD_MASK: u32 = (1 << SLAB_SHIFT) - 1;

const _: () = assert!(MAX_SLABS <= 1 << SLAB_BITS);

/// The device limits that constrain residency, taken from the created device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResidencyLimits {
    pub max_storage_buffer_binding_size: u64,
    pub max_buffer_size: u64,
}

impl ResidencyLimits {
    /// The largest single storage binding usable for an arena slab.
    pub fn max_slab_bytes(&self) -> u64 {
        self.max_storage_buffer_binding_size
            .min(self.max_buffer_size)
    }

    /// The largest slab in words, also bounded by what a slab address can name.
    pub fn max_slab_words(&self) -> u64 {
        (self.max_slab_bytes() / 4).min(SLAB_WORD_MASK as u64 + 1)
    }
}

/// Encodes a slab address.
pub const fn slab_address(slab: u32, word: u32) -> u32 {
    (slab << SLAB_SHIFT) | word
}

/// Splits a slab address into `(slab, word)`.
pub const fn split_slab_address(address: u32) -> (u32, u32) {
    (address >> SLAB_SHIFT, address & SLAB_WORD_MASK)
}

/// Where every logical page of one arena lives. See the [module docs](self).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlabPlan {
    /// The slab address of each logical page's first word.
    pub page_entries: Vec<u32>,
    /// Words in each slab.
    pub slab_words: Vec<u64>,
}

impl SlabPlan {
    pub fn slab_count(&self) -> usize {
        self.slab_words.len()
    }

    pub fn total_bytes(&self) -> u64 {
        self.slab_words.iter().sum::<u64>() * 4
    }

    /// Groups an arena's pages into one word vector per slab, following this plan.
    pub fn pack(&self, pages: &[Vec<u32>]) -> Vec<Vec<u32>> {
        let mut slabs: Vec<Vec<u32>> = self
            .slab_words
            .iter()
            .map(|&words| Vec::with_capacity(words as usize))
            .collect();
        for (page, &entry) in pages.iter().zip(&self.page_entries) {
            let (slab, word) = split_slab_address(entry);
            debug_assert_eq!(slabs[slab as usize].len(), word as usize);
            slabs[slab as usize].extend_from_slice(page);
        }
        slabs
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidencyError {
    /// One logical page is larger than any legal binding on this device.
    PageExceedsBinding {
        arena: &'static str,
        page_bytes: u64,
        limit_bytes: u64,
    },
    /// The arena needs more slabs than the shaders bind.
    ExceedsSlabs {
        arena: &'static str,
        required_bytes: u64,
        limit_bytes: u64,
    },
}

impl fmt::Display for ResidencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PageExceedsBinding {
                arena,
                page_bytes,
                limit_bytes,
            } => write!(
                f,
                "a {page_bytes} byte {arena} page exceeds the largest legal storage binding on \
                 this device ({limit_bytes} bytes); use smaller pages"
            ),
            Self::ExceedsSlabs {
                arena,
                required_bytes,
                limit_bytes,
            } => write!(
                f,
                "{arena} arena needs {required_bytes} bytes but {MAX_SLABS} slabs of at most \
                 {limit_bytes} bytes cannot hold it; streaming residency is not implemented yet"
            ),
        }
    }
}

impl std::error::Error for ResidencyError {}

/// Plans slabs for an arena whose pages hold `page_lengths` words each.
pub fn plan_slabs(
    arena: &'static str,
    page_lengths: impl IntoIterator<Item = usize>,
    limits: &ResidencyLimits,
) -> Result<SlabPlan, ResidencyError> {
    let max_words = limits.max_slab_words();
    let mut plan = SlabPlan {
        page_entries: Vec::new(),
        slab_words: Vec::new(),
    };
    let mut total_words = 0u64;
    for len in page_lengths {
        let len = len as u64;
        total_words += len;
        if len > max_words {
            return Err(ResidencyError::PageExceedsBinding {
                arena,
                page_bytes: len * 4,
                limit_bytes: max_words * 4,
            });
        }
        if plan
            .slab_words
            .last()
            .is_none_or(|&used| used + len > max_words)
        {
            plan.slab_words.push(0);
        }
        let slab = plan.slab_words.len() - 1;
        plan.page_entries
            .push(slab_address(slab as u32, plan.slab_words[slab] as u32));
        plan.slab_words[slab] += len;
    }
    if plan.slab_words.len() > MAX_SLABS {
        return Err(ResidencyError::ExceedsSlabs {
            arena,
            required_bytes: total_words * 4,
            limit_bytes: max_words * 4,
        });
    }
    // bindings may not be empty, so every arena has at least one slab
    if plan.slab_words.is_empty() {
        plan.slab_words.push(0);
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: ResidencyLimits = ResidencyLimits {
        max_storage_buffer_binding_size: 1024,
        max_buffer_size: 4096,
    };

    #[test]
    fn pages_pack_back_to_back_in_one_slab() {
        let plan = plan_slabs("topology", [10, 0, 6], &LIMITS).unwrap();
        assert_eq!(plan.page_entries, vec![0, 10, 10]);
        assert_eq!(plan.slab_words, vec![16]);
    }

    #[test]
    fn the_smaller_limit_bounds_each_slab() {
        // 1024 bytes = 256 words: two 200-word pages need two slabs
        let plan = plan_slabs("topology", [200, 200, 56], &LIMITS).unwrap();
        assert_eq!(plan.slab_count(), 2);
        assert_eq!(
            plan.page_entries,
            vec![slab_address(0, 0), slab_address(1, 0), slab_address(1, 200)]
        );
        assert_eq!(plan.slab_words, vec![200, 256]);
    }

    #[test]
    fn pages_never_straddle_slabs() {
        let lengths = [100, 100, 100, 30, 120, 7];
        let plan = plan_slabs("attributes", lengths, &LIMITS).unwrap();
        for (len, entry) in lengths.iter().zip(&plan.page_entries) {
            let (slab, word) = split_slab_address(*entry);
            assert!(word as u64 + *len as u64 <= plan.slab_words[slab as usize]);
            assert!(plan.slab_words[slab as usize] <= LIMITS.max_slab_words());
        }
        let pages: Vec<Vec<u32>> = lengths
            .iter()
            .enumerate()
            .map(|(i, &n)| vec![i as u32; n])
            .collect();
        let slabs = plan.pack(&pages);
        for (i, entry) in plan.page_entries.iter().enumerate() {
            let (slab, word) = split_slab_address(*entry);
            assert_eq!(
                &slabs[slab as usize][word as usize..word as usize + lengths[i]],
                &pages[i][..]
            );
        }
    }

    #[test]
    fn oversized_pages_and_slab_exhaustion_are_reported() {
        assert_eq!(
            plan_slabs("topology", [257], &LIMITS),
            Err(ResidencyError::PageExceedsBinding {
                arena: "topology",
                page_bytes: 1028,
                limit_bytes: 1024
            })
        );
        assert!(matches!(
            plan_slabs("topology", [256; MAX_SLABS + 1], &LIMITS),
            Err(ResidencyError::ExceedsSlabs { .. })
        ));
        assert!(plan_slabs("topology", [256; MAX_SLABS], &LIMITS).is_ok());
    }

    #[test]
    fn empty_arenas_still_get_a_slab() {
        let plan = plan_slabs("attributes", [], &LIMITS).unwrap();
        assert_eq!(plan.slab_count(), 1);
    }
}

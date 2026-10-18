//! Mapping logical arena pages onto resident GPU storage.
//!
//! The fast path packs every logical page of an arena into one contiguous storage binding that
//! fits the limits the device was *actually created with*. The shader then resolves a handle as
//! `page_base[page] + word_offset`, and a voxel root whose topology lives in one page resolves its
//! base once at root entry instead of per node.
//!
//! When an arena does not fit one binding, a multi-slab layout with cross-slab references is
//! required. That path is not implemented yet, so planning reports a precise error instead of
//! silently treating unloaded geometry as empty.

use std::fmt;

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
}

/// A single-slab layout: all pages packed back to back in one binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleSlabPlan {
    /// The word index in the slab where each logical page starts.
    pub page_bases: Vec<u32>,
    pub total_words: u64,
}

impl SingleSlabPlan {
    pub fn total_bytes(&self) -> u64 {
        self.total_words * 4
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidencyError {
    /// The arena needs a multi-slab layout on this device.
    ExceedsSingleBinding {
        arena: &'static str,
        required_bytes: u64,
        limit_bytes: u64,
    },
}

impl fmt::Display for ResidencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExceedsSingleBinding {
                arena,
                required_bytes,
                limit_bytes,
            } => write!(
                f,
                "{arena} arena needs {required_bytes} bytes but the largest legal storage binding \
                 on this device is {limit_bytes} bytes; the multi-slab residency path is not \
                 implemented yet"
            ),
        }
    }
}

impl std::error::Error for ResidencyError {}

/// Plans the single-slab fast path for an arena whose pages hold `page_lengths` words each.
pub fn plan_single_slab(
    arena: &'static str,
    page_lengths: impl IntoIterator<Item = usize>,
    limits: &ResidencyLimits,
) -> Result<SingleSlabPlan, ResidencyError> {
    let mut page_bases = Vec::new();
    let mut total_words = 0u64;
    for len in page_lengths {
        page_bases.push(total_words as u32);
        total_words += len as u64;
    }

    // keep at least one word so empty arenas still produce a valid binding
    let required_bytes = total_words.max(1) * 4;
    let limit_bytes = limits.max_slab_bytes();
    if required_bytes > limit_bytes || total_words > u32::MAX as u64 {
        return Err(ResidencyError::ExceedsSingleBinding {
            arena,
            required_bytes,
            limit_bytes,
        });
    }

    Ok(SingleSlabPlan {
        page_bases,
        total_words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: ResidencyLimits = ResidencyLimits {
        max_storage_buffer_binding_size: 1024,
        max_buffer_size: 4096,
    };

    #[test]
    fn pages_pack_back_to_back() {
        let plan = plan_single_slab("topology", [10, 0, 6], &LIMITS).unwrap();
        assert_eq!(plan.page_bases, vec![0, 10, 10]);
        assert_eq!(plan.total_words, 16);
    }

    #[test]
    fn the_smaller_limit_wins() {
        // 1024 bytes = 256 words fits the binding limit exactly
        assert!(plan_single_slab("topology", [256], &LIMITS).is_ok());
        let err = plan_single_slab("topology", [257], &LIMITS).unwrap_err();
        assert_eq!(
            err,
            ResidencyError::ExceedsSingleBinding {
                arena: "topology",
                required_bytes: 1028,
                limit_bytes: 1024
            }
        );
    }
}

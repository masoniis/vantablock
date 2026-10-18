// Portable 64-bit occupancy masks as two u32 halves. Mirrors crates/engine/voxel/src/mask.rs.

const NODE_WORDS: u32 = 4u;

// Number of occupied children strictly below child k (k in 0..64).
fn prefix_count(mask: vec2<u32>, k: u32) -> u32 {
    let high = k >= 32u;
    let below = (1u << (k & 31u)) - 1u;
    let lower = countOneBits(select(mask.x & below, mask.x, high));
    let upper = countOneBits(select(0u, mask.y & below, high));
    return lower + upper;
}

// Word offset of child k's Node64 header within its contiguous child block.
fn child_word_offset(mask: vec2<u32>, k: u32) -> u32 {
    return prefix_count(mask, k) * NODE_WORDS;
}

fn mask_bit(mask: vec2<u32>, k: u32) -> bool {
    let word = select(mask.x, mask.y, k >= 32u);
    return ((word >> (k & 31u)) & 1u) != 0u;
}

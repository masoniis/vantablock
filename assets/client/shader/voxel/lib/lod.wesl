// Footprint LOD filtering. Mirrors crates/engine/voxel/src/lod.rs: the packed summary record, the
// fitted SGGX opacity, and the footprint rule. The traversal that uses them is
// `trace_tree64_filtered` in tree64.wesl, compiled in only with the `footprint_lod` feature.

// Low 31 bits of a node header's `lod_meta`, and the index meaning "no summary baked".
const LOD_INDEX_MASK: u32 = 0x7FFFFFFFu;
const LOD_MISSING: u32 = 0x7FFFFFFFu;

// One ray cone and opacity threshold (`voxel::lod::FootprintFilter`). The footprint radius at
// distance `t` is `radius + spread * t` in world units.
struct FootprintFilter {
    radius: f32,
    spread: f32,
    threshold: f32,
};

// A cone of zero width: nothing fits inside it, so traversal is exact.
fn exact_footprint() -> FootprintFilter {
    return FootprintFilter(0.0, 0.0, 0.0);
}

// Whether a cell `size` voxels on a side, entered at `t`, fits inside the footprint of a ray
// whose direction has length `voxels_per_unit`.
fn footprint_covers(cone: FootprintFilter, size: f32, t: f32, voxels_per_unit: f32) -> bool {
    return size <= 2.0 * (cone.radius + cone.spread * t) * voxels_per_unit;
}

// The fitted SGGX tensor's `min(sqrt(max(w^T S w, 0)), 1)` with `w = normalize(d)`,
// `S_aa = c_a^2`, and `S_ab = r_ab c_a c_b` (`LodSummary::opacity`): the chance of a hit for a ray
// entering the summary's occupied box.
fn lod_opacity(record: vec4<u32>, d: vec3<f32>) -> f32 {
    let a = normalize(d) * lod_coverage(record);
    let r = lod_correlation(record);
    let q = dot(a, a) + 2.0 * (a.x * a.y * r.x + a.x * a.z * r.y + a.y * a.z * r.z);
    return min(sqrt(max(q, 0.0)), 1.0);
}

// The fitted tensor's coverage of the summary's occupied box along X, Y, and Z (`sqrt(S_aa)`):
// f16s in word 0 and the low half of word 1.
fn lod_coverage(record: vec4<u32>) -> vec3<f32> {
    return vec3<f32>(unpack2x16float(record.x), unpack2x16float(record.y).x);
}

// The fitted tensor's coverage of the node's faces along X, Y, and Z (the spec's axis coverage):
// the box faces' coverage times their share of the node's faces.
fn lod_node_coverage(record: vec4<u32>) -> vec3<f32> {
    let w = record.w;
    let q_lo = vec3<f32>(vec3<u32>(w, w >> 4u, w >> 8u) & vec3<u32>(0xFu));
    let q_hi = vec3<f32>(vec3<u32>(w >> 12u, w >> 16u, w >> 20u) & vec3<u32>(0xFu)) + 1.0;
    let e = (q_hi - q_lo) / 16.0;
    return lod_coverage(record) * vec3<f32>(e.y * e.z, e.x * e.z, e.x * e.y);
}

// The summary's representative material (palette index): the high half of word 1.
fn lod_material(record: vec4<u32>) -> u32 {
    return record.y >> 16u;
}

// The box a filtered hit fills (`LodSummary::filtered_box`): the occupied box of the node whose
// minimum corner is `lo`, `size` voxels on a side. Word 3 holds the box's corners in sixteenths
// of the node's edge, 4 bits each (minimum x, y, z, then maximum minus one).
fn lod_filtered_box(record: vec4<u32>, lo: vec3<f32>, size: f32) -> array<vec3<f32>, 2> {
    let w = record.w;
    let q_lo = vec3<f32>(vec3<u32>(w, w >> 4u, w >> 8u) & vec3<u32>(0xFu));
    let q_hi = vec3<f32>(vec3<u32>(w >> 12u, w >> 16u, w >> 20u) & vec3<u32>(0xFu)) + 1.0;
    let scale = size / 16.0;
    return array<vec3<f32>, 2>(lo + q_lo * scale, lo + q_hi * scale);
}

// The fitted tensor's correlations `r_xy`, `r_xz`, `r_yz` (`S_ab / sqrt(S_aa S_bb)`): word 2
// holds them as 10 bit offset snorms.
fn lod_correlation(record: vec4<u32>) -> vec3<f32> {
    let q = vec3<u32>(record.z, record.z >> 10u, record.z >> 20u) & vec3<u32>(0x3FFu);
    return (vec3<f32>(q) - 511.0) / 511.0;
}

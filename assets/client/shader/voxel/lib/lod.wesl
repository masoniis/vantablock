// Footprint LOD filtering. Mirrors crates/engine/voxel/src/lod.rs: the packed summary record, the
// axis-coverage opacity proxy, and the footprint rule. The traversal that uses them is
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

// `min(sqrt(sum(d_axis^2 * coverage_axis^2)), 1)` with `d` normalized (`LodSummary::opacity`):
// the chance of a hit for a ray entering the summary's occupied box.
fn lod_opacity(record: vec4<u32>, d: vec3<f32>) -> f32 {
    return min(length(normalize(d) * lod_coverage(record)), 1.0);
}

// The summary's projected coverage of its occupied box's faces along X, Y, and Z: f16s in word 0
// and the low half of word 1.
fn lod_coverage(record: vec4<u32>) -> vec3<f32> {
    return vec3<f32>(unpack2x16float(record.x), unpack2x16float(record.y).x);
}

// The summary's projected coverage of the node's faces (the spec's axis coverage): the box faces'
// coverage times their share of the node's faces.
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

// The summary's mean outward normal in module space (`LodSummary::unpack`): word 2 holds a 12+12
// bit octahedral encoding and the 8 bit normal spread. Zero when the exposed faces cancel out
// (full spread). Shading doesn't use it: `filtered_face_light` in direct_lighting.wesl averages
// the camera-facing axis faces by coverage, which the mean normal can't describe.
fn lod_normal(record: vec4<u32>) -> vec3<f32> {
    if (record.z >> 24u) == 255u {
        return vec3<f32>(0.0);
    }
    let e = vec2<f32>(f32(record.z & 0xFFFu), f32((record.z >> 12u) & 0xFFFu)) / 4095.0 * 2.0 - 1.0;
    // `e` is never zero (4095 is odd), so the sign is never ambiguous
    let z = 1.0 - abs(e.x) - abs(e.y);
    var xy = e;
    if z < 0.0 {
        xy = (vec2<f32>(1.0) - abs(e.yx)) * select(vec2<f32>(-1.0), vec2<f32>(1.0), e >= vec2<f32>(0.0));
    }
    return normalize(vec3<f32>(xy, z));
}

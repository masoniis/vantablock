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

// `clamp(sum(|d_axis| * coverage_axis), 0, 1)` with `d` normalized (`LodSummary::opacity`). Word
// 0 holds volume and X coverage, word 1 Y and Z coverage, all as f16.
fn lod_opacity(record: vec4<u32>, d: vec3<f32>) -> f32 {
    let coverage = vec3<f32>(unpack2x16float(record.x).y, unpack2x16float(record.y));
    return clamp(dot(abs(normalize(d)), coverage), 0.0, 1.0);
}

// The summary's representative material (palette index).
fn lod_material(record: vec4<u32>) -> u32 {
    return record.w & 0xFFFFu;
}

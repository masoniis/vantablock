// GPU conformance kernels: run the production mask, 64-tree, and scene code on the real adapter
// so the client's tests can compare it against the CPU reference. Groups 0 and 1 are the
// production geometry and scene groups; the test's own buffers live in group 2.

import super::super::lib::mask::{prefix_count, child_word_offset};
import super::super::lib::ray::sanitize_direction;
import super::super::lib::tree64::{trace_tree64, Tree64Root};
import super::super::lib::scene::trace_scene;

struct MaskCase {
    mask: vec2<u32>,
    k: u32,
    pad: u32,
};

struct RayCase {
    origin: vec3<f32>,
    t_min: f32,
    dir: vec3<f32>,
    t_max: f32,
};

struct RayResult {
    hit: u32,
    t: f32,
    cell: u32,
    attribute_handle: u32,
};

struct RootParams {
    node: u32,
    levels: u32,
    page_base: u32,
    single_page: u32,
};

@group(2) @binding(0) var<storage, read> mask_cases: array<MaskCase>;
@group(2) @binding(1) var<storage, read_write> mask_results: array<vec2<u32>>;
@group(2) @binding(2) var<storage, read> ray_cases: array<RayCase>;
@group(2) @binding(3) var<storage, read_write> ray_results: array<RayResult>;
@group(2) @binding(4) var<uniform> root_params: RootParams;

@compute @workgroup_size(64, 1, 1)
fn masks(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&mask_cases) {
        return;
    }
    let c = mask_cases[id.x];
    mask_results[id.x] = vec2<u32>(prefix_count(c.mask, c.k), child_word_offset(c.mask, c.k));
}

@compute @workgroup_size(64, 1, 1)
fn rays(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&ray_cases) {
        return;
    }
    let c = ray_cases[id.x];
    var root: Tree64Root;
    root.node = root_params.node;
    root.levels = root_params.levels;
    root.page_base = root_params.page_base;
    root.single_page = root_params.single_page != 0u;
    let hit = trace_tree64(root, c.origin, sanitize_direction(c.dir), c.t_min, c.t_max);
    var result: RayResult;
    result.hit = select(0u, 1u, hit.hit);
    result.t = hit.t;
    result.cell = hit.cell.x | (hit.cell.y << 10u) | (hit.cell.z << 20u);
    result.attribute_handle = hit.attribute_handle;
    ray_results[id.x] = result;
}

// Whole-scene queries: `attribute_handle` carries the hit frame instead.
@compute @workgroup_size(64, 1, 1)
fn scene_rays(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&ray_cases) {
        return;
    }
    let c = ray_cases[id.x];
    let hit = trace_scene(c.origin, c.dir, c.t_min, c.t_max);
    var result: RayResult;
    result.hit = select(0u, 1u, hit.hit);
    result.t = hit.t;
    result.cell = hit.voxel.cell.x | (hit.voxel.cell.y << 10u) | (hit.voxel.cell.z << 20u);
    result.attribute_handle = hit.frame;
    ray_results[id.x] = result;
}

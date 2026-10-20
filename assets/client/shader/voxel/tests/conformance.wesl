// GPU conformance kernels: run the production mask, hierarchy, and scene code on the real adapter
// so the client's tests can compare it against the CPU reference. Groups 0 and 1 are the
// production geometry and scene groups; the test's own buffers live in group 2.

import super::super::lib::mask::{prefix_count, child_word_offset};
import super::super::lib::ray::sanitize_direction;
import super::super::lib::dda::{VoxelRoot, VoxelHit};
import super::super::lib::tree64::{trace_tree64, trace_tree64_filtered};
import super::super::lib::lod::{FootprintFilter, lod_material, lod_normal};
import super::super::lib::geometry::{lod_record_load, voxel_material};
import super::super::lib::octree::trace_octree;
import super::super::lib::scene::{trace_scene, trace_scene_filtered, previous_world_position};

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
    // bit 0: single page, bit 1: octree
    flags: u32,
    // occupied bounds in module voxels
    bounds_min: vec3<u32>,
    pad0: u32,
    bounds_max: vec3<u32>,
    pad1: u32,
};

@group(2) @binding(0) var<storage, read> mask_cases: array<MaskCase>;
@group(2) @binding(1) var<storage, read_write> mask_results: array<vec2<u32>>;
@group(2) @binding(2) var<storage, read> ray_cases: array<RayCase>;
@group(2) @binding(3) var<storage, read_write> ray_results: array<RayResult>;
@group(2) @binding(4) var<uniform> root_params: RootParams;
@group(2) @binding(5) var<storage, read_write> motion_results: array<vec4<f32>>;

// `voxel::lod::FootprintFilter`, padded for the uniform.
struct FilterParams {
    radius: f32,
    spread: f32,
    threshold: f32,
    pad: u32,
};

struct FilteredRayResult {
    hit: u32,
    t: f32,
    cell: u32,
    lod: u32,
    material: u32,
    steps: u32,
    // the hit frame, for whole-scene queries
    frame: u32,
    // a filtered hit's LOD record index and its decoded summary normal (bare 64-tree queries)
    record: u32,
    normal: vec3<f32>,
    pad: u32,
};

@group(2) @binding(6) var<uniform> filter_params: FilterParams;
@group(2) @binding(7) var<storage, read_write> filtered_results: array<FilteredRayResult>;

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
    var root: VoxelRoot;
    root.node = root_params.node;
    root.levels = root_params.levels;
    root.page_base = root_params.page_base;
    root.single_page = (root_params.flags & 1u) != 0u;
    var hit: VoxelHit;
    if (root_params.flags & 2u) != 0u {
        hit = trace_octree(root, c.origin, sanitize_direction(c.dir), c.t_min, c.t_max);
    } else {
        hit = trace_tree64(root, c.origin, sanitize_direction(c.dir), c.t_min, c.t_max);
    }
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

// Whole-scene queries that also reconstruct each hit's previous world position (xyz, w = 1 for
// hits) from the hit distance and resolved frame, as temporal reprojection will.
@compute @workgroup_size(64, 1, 1)
fn scene_motion(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&ray_cases) {
        return;
    }
    let c = ray_cases[id.x];
    let hit = trace_scene(c.origin, c.dir, c.t_min, c.t_max);
    if !hit.hit {
        motion_results[id.x] = vec4<f32>(0.0);
        return;
    }
    let p = c.origin + c.dir * hit.t;
    motion_results[id.x] = vec4<f32>(previous_world_position(hit.frame, p), 1.0);
}

// 64-tree queries with footprint LOD filtering, reporting what `tree64::trace_filtered` does: the
// level, the material (the voxel's, or the LOD summary's), and the DDA steps.
@ if(footprint_lod)
@compute @workgroup_size(64, 1, 1)
fn filtered_rays(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&ray_cases) {
        return;
    }
    let c = ray_cases[id.x];
    var root: VoxelRoot;
    root.node = root_params.node;
    root.levels = root_params.levels;
    root.page_base = root_params.page_base;
    root.single_page = (root_params.flags & 1u) != 0u;
    root.bounds_min = vec3<i32>(root_params.bounds_min);
    root.bounds_max = vec3<i32>(root_params.bounds_max);
    let cone = FootprintFilter(filter_params.radius, filter_params.spread, filter_params.threshold);
    let hit = trace_tree64_filtered(root, c.origin, sanitize_direction(c.dir), c.t_min, c.t_max, cone);
    var result: FilteredRayResult;
    result.hit = select(0u, 1u, hit.hit);
    result.steps = hit.steps;
    if hit.hit {
        result.t = hit.t;
        result.cell = hit.cell.x | (hit.cell.y << 10u) | (hit.cell.z << 20u);
        result.lod = hit.lod;
        if hit.lod == 0u {
            result.material = voxel_material(hit.attribute_handle);
        } else {
            let record = lod_record_load(hit.attribute_handle);
            result.material = lod_material(record);
            result.record = hit.attribute_handle;
            result.normal = lod_normal(record);
        }
    }
    filtered_results[id.x] = result;
}

// Whole-scene queries with footprint LOD filtering (`trace_scene_filtered`), reporting the hit
// frame, cell, and level. The world-space cone reaches every frame's 64-tree; octree frames stay
// exact.
@ if(footprint_lod)
@compute @workgroup_size(64, 1, 1)
fn filtered_scene_rays(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&ray_cases) {
        return;
    }
    let c = ray_cases[id.x];
    let cone = FootprintFilter(filter_params.radius, filter_params.spread, filter_params.threshold);
    let hit = trace_scene_filtered(c.origin, c.dir, c.t_min, c.t_max, cone);
    var result: FilteredRayResult;
    result.hit = select(0u, 1u, hit.hit);
    if hit.hit {
        result.t = hit.t;
        result.cell = hit.voxel.cell.x | (hit.voxel.cell.y << 10u) | (hit.voxel.cell.z << 20u);
        result.lod = hit.voxel.lod;
        result.frame = hit.frame;
    }
    filtered_results[id.x] = result;
}

// Shared hierarchical DDA rules for every local voxel hierarchy. Mirrors crates/engine/voxel/src/hierarchy/mod.rs:
// boundary distances are recomputed from integer cells, ties pick the lowest axis, and entry faces
// come from the same plane distances as the slab test.

import super::geometry::{topology_page_base, WORD_OFFSET_MASK};

const MAX_TRAVERSAL_STEPS: u32 = 4096u;
const FACE_INSIDE: u32 = 6u;

// A baked local hierarchy root, as either hierarchy reads it.
struct VoxelRoot {
    // logical handle of the root node record
    node: u32,
    // 64-tree levels, or octree levels above the bricks
    levels: u32,
    // slab base of the root's page, valid when single_page
    page_base: u32,
    single_page: bool,
};

struct VoxelHit {
    hit: bool,
    t: f32,
    cell: vec3<u32>,
    // index within the hit voxel's aligned 4x4x4 block
    terminal_index: u32,
    face: u32,
    // logical handle of the hit voxel's attribute word
    attribute_handle: u32,
    steps: u32,
};

// Resolves a topology handle. Roots whose whole tree lives in one page resolve their page once at
// root entry; others go through the page table.
fn node_address(root: VoxelRoot, node_handle: u32) -> u32 {
    if root.single_page {
        return root.page_base + (node_handle & WORD_OFFSET_MASK);
    }
    return topology_page_base(node_handle >> 24u) + (node_handle & WORD_OFFSET_MASK);
}

fn next_boundary(origin: vec3<i32>, cell: vec3<i32>, size: i32, far: vec3<i32>, o: vec3<f32>, inv: vec3<f32>) -> vec3<f32> {
    return (vec3<f32>(origin + (cell + far) * size) - o) * inv;
}

// One-hot axis of the nearest boundary; ties pick the lowest axis, matching the CPU.
fn nearest_axis(t: vec3<f32>) -> vec3<i32> {
    if t.x <= t.y && t.x <= t.z {
        return vec3<i32>(1, 0, 0);
    }
    if t.y <= t.z {
        return vec3<i32>(0, 1, 0);
    }
    return vec3<i32>(0, 0, 1);
}

fn entry_face(lo: vec3<i32>, size: i32, o: vec3<f32>, inv: vec3<f32>, step: vec3<i32>, te: f32, t_start: f32) -> u32 {
    if te < t_start {
        return FACE_INSIDE;
    }
    let t0 = (vec3<f32>(lo) - o) * inv;
    let t1 = (vec3<f32>(lo + vec3<i32>(size)) - o) * inv;
    let near = min(t0, t1);
    // stepping up enters through the min face, whose outward normal points down the axis
    let down = step < vec3<i32>(0);
    if near.x >= near.y && near.x >= near.z {
        return select(0u, 1u, down.x);
    }
    if near.y >= near.z {
        return select(2u, 3u, down.y);
    }
    return select(4u, 5u, down.z);
}

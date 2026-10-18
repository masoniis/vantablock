// The 64-tree hierarchical DDA. Mirrors `trace` in crates/voxel/src/hierarchy/tree64.rs line for
// line: entry cells come from plane distances (never from flooring a reconstructed position),
// boundary distances are recomputed from integer cells, and ties pick the lowest axis.

import super::geometry::{topology, topology_page_base, WORD_OFFSET_MASK};
import super::mask::{prefix_count, child_word_offset, mask_bit};
import super::ray::slab;

const MAX_LEVELS: u32 = 5u;
const MAX_TRAVERSAL_STEPS: u32 = 4096u;
const FACE_INSIDE: u32 = 6u;

struct Tree64Root {
    // logical handle of the root node header
    node: u32,
    levels: u32,
    // slab base of the root's page, valid when single_page
    page_base: u32,
    single_page: bool,
};

struct VoxelHit {
    hit: bool,
    t: f32,
    cell: vec3<u32>,
    terminal_index: u32,
    face: u32,
    // logical handle of the hit voxel's attribute word
    attribute_handle: u32,
    steps: u32,
};

// Resolves a topology handle. Roots whose whole tree lives in one page resolve their page once at
// root entry; others go through the page table.
fn node_address(root: Tree64Root, node_handle: u32) -> u32 {
    if root.single_page {
        return root.page_base + (node_handle & WORD_OFFSET_MASK);
    }
    return topology_page_base(node_handle >> 24u) + (node_handle & WORD_OFFSET_MASK);
}

// The child cell the ray occupies at `t`, by counting the node's internal planes it has crossed.
// Plane distances grow monotonically in ray order, so counting every crossed plane equals the CPU
// reference's count-until-first-uncrossed. Written with whole-vector operations: dynamically
// indexing vector components forces them out of registers on most GPUs.
fn entry_cell(origin: vec3<i32>, size: i32, o: vec3<f32>, inv: vec3<f32>, step: vec3<i32>, t: f32) -> vec3<i32> {
    let up = step > vec3<i32>(0);
    var crossed = vec3<i32>(0);
    for (var j = 1; j < 4; j++) {
        let plane = select(vec3<i32>(4 - j), vec3<i32>(j), up);
        let t_plane = (vec3<f32>(origin + plane * size) - o) * inv;
        crossed += select(vec3<i32>(0), vec3<i32>(1), t_plane <= vec3<f32>(t));
    }
    return select(vec3<i32>(3) - crossed, crossed, up);
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

fn cell_of(k: u32) -> vec3<i32> {
    return vec3<i32>(i32(k & 3u), i32((k >> 2u) & 3u), i32(k >> 4u));
}

// Traces a ray in module voxel space. `d` must already be sanitized.
//
// The current level lives in registers. Each parent level keeps only its mask and child block
// handle on the stack; the cell taken at every level is packed six bits per level into `path`,
// and a parent's origin and boundary distances are recomputed on pop with the same formulas that
// produced them, so the results are bit-identical to keeping them.
fn trace_tree64(root: Tree64Root, o: vec3<f32>, d: vec3<f32>, t_min: f32, t_max: f32) -> VoxelHit {
    var result: VoxelHit;
    result.hit = false;
    result.steps = 0u;

    let inv = 1.0 / d;
    let extent = i32(1u << (2u * root.levels));
    let root_t = slab(vec3<f32>(0.0), vec3<f32>(f32(extent)), o, inv);
    let t_start = max(root_t.x, t_min);
    let t_end = min(root_t.y, t_max);
    if t_start > t_end {
        return result;
    }

    let root_address = node_address(root, root.node);
    var mask = vec2<u32>(topology[root_address], topology[root_address + 1u]);
    if mask.x == 0u && mask.y == 0u {
        return result;
    }
    var child_ref = topology[root_address + 2u];

    let step = select(vec3<i32>(1), vec3<i32>(-1), d < vec3<f32>(0.0));
    // a cell's far boundary is at cell + 1 when stepping up, cell when stepping down
    let far = (step + vec3<i32>(1)) / 2;
    let last_level = root.levels - 1u;

    var stack_mask: array<vec2<u32>, 5>;
    var stack_ref: array<u32, 5>;
    var path = 0u;
    var depth = 0u;
    var size = extent / 4;
    var origin = vec3<i32>(0);
    var cell = entry_cell(origin, size, o, inv, step, t_start);
    var t_next = next_boundary(origin, cell, size, far, o, inv);

    var steps = 0u;
    loop {
        if steps >= MAX_TRAVERSAL_STEPS {
            break;
        }
        steps += 1u;

        if any(cell < vec3<i32>(0)) || any(cell > vec3<i32>(3)) {
            // left this node: resume the parent past the cell we were in
            if depth == 0u {
                break;
            }
            depth -= 1u;
            size *= 4;
            cell = cell_of((path >> (6u * depth)) & 63u);
            origin -= cell * size;
            mask = stack_mask[depth];
            child_ref = stack_ref[depth];
            t_next = next_boundary(origin, cell, size, far, o, inv);
            cell += step * nearest_axis(t_next);
            t_next = next_boundary(origin, cell, size, far, o, inv);
            continue;
        }

        let k = u32(cell.x + 4 * cell.y + 16 * cell.z);
        if mask_bit(mask, k) {
            let lo = origin + cell * size;
            let tt = slab(vec3<f32>(lo), vec3<f32>(lo + vec3<i32>(size)), o, inv);

            if depth == last_level {
                // a voxel; only report it if the ray genuinely touches it within range
                if tt.x <= tt.y && tt.y >= t_start && tt.x <= t_end {
                    result.hit = true;
                    result.t = max(tt.x, t_start);
                    result.cell = vec3<u32>(lo);
                    result.terminal_index = k;
                    result.face = entry_face(lo, size, o, inv, step, tt.x, t_start);
                    // offsets stay within the block's page, so this is still a valid handle
                    result.attribute_handle = child_ref + prefix_count(mask, k);
                    result.steps = steps;
                    return result;
                }
            } else {
                let address = node_address(root, child_ref + child_word_offset(mask, k));
                stack_mask[depth] = mask;
                stack_ref[depth] = child_ref;
                path = (path & ~(63u << (6u * depth))) | (k << (6u * depth));
                depth += 1u;
                size /= 4;
                origin = lo;
                mask = vec2<u32>(topology[address], topology[address + 1u]);
                child_ref = topology[address + 2u];
                cell = entry_cell(origin, size, o, inv, step, max(tt.x, t_start));
                t_next = next_boundary(origin, cell, size, far, o, inv);
                continue;
            }
        }

        cell += step * nearest_axis(t_next);
        t_next = next_boundary(origin, cell, size, far, o, inv);
    }

    result.steps = steps;
    return result;
}

// The 64-tree hierarchical DDA. Mirrors `trace` and `trace_filtered` in
// crates/engine/voxel/src/hierarchy/tree64.rs line for line, following the shared rules in
// dda.wesl: entry cells come from plane distances (never from flooring a reconstructed position),
// boundary distances are recomputed from integer cells, and ties pick the lowest axis.

import super::geometry::{topology_load, lod_record_load};
import super::lod::{FootprintFilter, LOD_INDEX_MASK, LOD_MISSING, exact_footprint, footprint_covers, lod_opacity};
import super::mask::{prefix_count, child_word_offset, mask_bit};
import super::ray::slab;
import super::dda::{VoxelRoot, VoxelHit, MAX_TRAVERSAL_STEPS, node_address, next_boundary, nearest_axis, entry_face};

const MAX_LEVELS: u32 = 5u;

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

fn cell_of(k: u32) -> vec3<i32> {
    return vec3<i32>(i32(k & 3u), i32((k >> 2u) & 3u), i32(k >> 4u));
}

// Parent-level state kept while descending. The default keeps each parent's mask and child block
// handle in a short private stack. The `stackless` variant keeps nothing and re-walks the packed
// path from the root on every pop, trading a few (cached) node fetches for registers; both
// produce bit-identical results. See gpu_scene_throughput for how they compare.
@ if(!stackless)
struct ParentStack {
    mask: array<vec2<u32>, MAX_LEVELS>,
    child_ref: array<u32, MAX_LEVELS>,
};

@ if(stackless)
struct ParentStack {
    unused: u32,
};

@ if(!stackless)
fn push_parent(stack: ptr<function, ParentStack>, depth: u32, mask: vec2<u32>, child_ref: u32) {
    (*stack).mask[depth] = mask;
    (*stack).child_ref[depth] = child_ref;
}

@ if(stackless)
fn push_parent(stack: ptr<function, ParentStack>, depth: u32, mask: vec2<u32>, child_ref: u32) {}

// The header of the node at `depth` on the current path: (mask.x, mask.y, child_ref).
@ if(!stackless)
fn parent_node(stack: ptr<function, ParentStack>, root: VoxelRoot, path: u32, depth: u32) -> vec3<u32> {
    return vec3<u32>((*stack).mask[depth], (*stack).child_ref[depth]);
}

@ if(stackless)
fn parent_node(stack: ptr<function, ParentStack>, root: VoxelRoot, path: u32, depth: u32) -> vec3<u32> {
    var address = node_address(root, root.node);
    var node = vec3<u32>(topology_load(address), topology_load(address + 1u), topology_load(address + 2u));
    for (var level = 0u; level < depth; level++) {
        let k = (path >> (6u * level)) & 63u;
        address = node_address(root, node.z + child_word_offset(node.xy, k));
        node = vec3<u32>(topology_load(address), topology_load(address + 1u), topology_load(address + 2u));
    }
    return node;
}

// What to do with an occupied child about to be descended into.
const FOOTPRINT_DESCEND: u32 = 0u;
// Resolve it from its LOD summary: the ray hits the node's box.
const FOOTPRINT_OPAQUE: u32 = 1u;
// Resolve it from its LOD summary: the ray passes through.
const FOOTPRINT_CLEAR: u32 = 2u;

// The footprint decision for the occupied child whose header is at `address`, with cells `size`
// voxels on a side, entered at `t`: `(decision, LOD record index)`. Without the `footprint_lod`
// feature traversal always descends, so the filter costs the exact kernels nothing.
@ if(footprint_lod)
fn footprint_decision(cone: FootprintFilter, address: u32, size: i32, t: f32, d: vec3<f32>, voxels_per_unit: f32) -> vec2<u32> {
    if !footprint_covers(cone, f32(size), t, voxels_per_unit) {
        return vec2<u32>(FOOTPRINT_DESCEND, 0u);
    }
    let index = topology_load(address + 3u) & LOD_INDEX_MASK;
    if index == LOD_MISSING {
        return vec2<u32>(FOOTPRINT_DESCEND, 0u);
    }
    let opaque = cone.threshold < lod_opacity(lod_record_load(index), d);
    return vec2<u32>(select(FOOTPRINT_CLEAR, FOOTPRINT_OPAQUE, opaque), index);
}

@ if(!footprint_lod)
fn footprint_decision(cone: FootprintFilter, address: u32, size: i32, t: f32, d: vec3<f32>, voxels_per_unit: f32) -> vec2<u32> {
    return vec2<u32>(FOOTPRINT_DESCEND, 0u);
}

// Traces a ray in module voxel space. `d` must already be sanitized.
fn trace_tree64(root: VoxelRoot, o: vec3<f32>, d: vec3<f32>, t_min: f32, t_max: f32) -> VoxelHit {
    return trace_tree64_filtered(root, o, d, t_min, t_max, exact_footprint());
}

// `trace_tree64` with footprint LOD filtering (`footprint_lod` builds only; elsewhere it is
// exact): an occupied node whose cell fits inside the `cone`'s footprint is resolved from its LOD
// summary instead of being descended. A filtered hit reports the node's aligned corner as `cell`,
// its level as `lod`, and its LOD record index as `attribute_handle`.
//
// The current level lives in registers. The cell taken at every level is packed six bits per
// level into `path`, a parent's header comes back from `parent_node`, and its origin and boundary
// distances are recomputed on pop with the same formulas that produced them, so the results are
// bit-identical to keeping them.
fn trace_tree64_filtered(root: VoxelRoot, o: vec3<f32>, d: vec3<f32>, t_min: f32, t_max: f32, cone: FootprintFilter) -> VoxelHit {
    var result: VoxelHit;
    result.hit = false;
    result.steps = 0u;

    let inv = 1.0 / d;
    let voxels_per_unit = length(d);
    let extent = i32(1u << (2u * root.levels));
    let root_t = slab(vec3<f32>(0.0), vec3<f32>(f32(extent)), o, inv);
    let t_start = max(root_t.x, t_min);
    let t_end = min(root_t.y, t_max);
    if t_start > t_end {
        return result;
    }

    let root_address = node_address(root, root.node);
    var mask = vec2<u32>(topology_load(root_address), topology_load(root_address + 1u));
    if mask.x == 0u && mask.y == 0u {
        return result;
    }
    var child_ref = topology_load(root_address + 2u);

    let step = select(vec3<i32>(1), vec3<i32>(-1), d < vec3<f32>(0.0));
    // a cell's far boundary is at cell + 1 when stepping up, cell when stepping down
    let far = (step + vec3<i32>(1)) / 2;
    let last_level = root.levels - 1u;

    var parents: ParentStack;
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
            let parent = parent_node(&parents, root, path, depth);
            mask = parent.xy;
            child_ref = parent.z;
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
                let footprint = footprint_decision(cone, address, size, max(tt.x, t_start), d, voxels_per_unit);
                if footprint.x == FOOTPRINT_DESCEND {
                    push_parent(&parents, depth, mask, child_ref);
                    path = (path & ~(63u << (6u * depth))) | (k << (6u * depth));
                    depth += 1u;
                    size /= 4;
                    origin = lo;
                    mask = vec2<u32>(topology_load(address), topology_load(address + 1u));
                    child_ref = topology_load(address + 2u);
                    cell = entry_cell(origin, size, o, inv, step, max(tt.x, t_start));
                    t_next = next_boundary(origin, cell, size, far, o, inv);
                    continue;
                }
                // the summary stands in for the whole subtree: hit it or pass through it
                if footprint.x == FOOTPRINT_OPAQUE && tt.x <= tt.y && tt.y >= t_start && tt.x <= t_end {
                    result.hit = true;
                    result.t = max(tt.x, t_start);
                    result.cell = vec3<u32>(lo);
                    result.terminal_index = k;
                    result.face = entry_face(lo, size, o, inv, step, tt.x, t_start);
                    result.attribute_handle = footprint.y;
                    result.steps = steps;
                    result.lod = last_level - depth;
                    return result;
                }
            }
        }

        cell += step * nearest_axis(t_next);
        t_next = next_boundary(origin, cell, size, far, o, inv);
    }

    result.steps = steps;
    return result;
}

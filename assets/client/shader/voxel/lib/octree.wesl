// The 8-way octree with 8x8x8 bricks, the comparison hierarchy. Mirrors `trace` in
// crates/engine/voxel/src/hierarchy/octree.rs line for line, following the shared rules in dda.wesl.
//
// node  (2 words):  child mask (bits 0..8) | brick-children tag (bit 31), child block handle
// brick (17 words): occupancy (16 words, voxel x + 8y + 64z), attribute block handle

import super::geometry::topology_load;
import super::ray::slab;
import super::dda::{VoxelRoot, VoxelHit, MAX_TRAVERSAL_STEPS, node_address, next_boundary, nearest_axis, entry_face};

const OCTREE_NODE_WORDS: u32 = 2u;
const BRICK_MASK_WORDS: u32 = 16u;
const BRICK_WORDS: u32 = 17u;
const OCTREE_CHILD_MASK: u32 = 0xFFu;
const MAX_OCTREE_DEPTH: u32 = 7u;

// The cell of a node with `cells` cells per axis that the ray occupies at `t`, by counting the
// internal planes it has crossed (see `entry_cell` in tree64.wesl).
fn entry_cell_n(origin: vec3<i32>, size: i32, cells: i32, o: vec3<f32>, inv: vec3<f32>, step: vec3<i32>, t: f32) -> vec3<i32> {
    let up = step > vec3<i32>(0);
    var crossed = vec3<i32>(0);
    for (var j = 1; j < cells; j++) {
        let plane = select(vec3<i32>(cells - j), vec3<i32>(j), up);
        let t_plane = (vec3<f32>(origin + plane * size) - o) * inv;
        crossed += select(vec3<i32>(0), vec3<i32>(1), t_plane <= vec3<f32>(t));
    }
    return select(vec3<i32>(cells - 1) - crossed, crossed, up);
}

fn octant_of(k: u32) -> vec3<i32> {
    return vec3<i32>(i32(k & 1u), i32((k >> 1u) & 1u), i32(k >> 2u));
}

// Traces a ray in module voxel space. `d` must already be sanitized.
//
// Like the 64-tree, parent levels keep only their mask and child block handle on the stack, and
// the octant taken at each level is packed three bits per level into `path`. On the brick level
// `child_ref` holds the brick record's handle.
fn trace_octree(root: VoxelRoot, o: vec3<f32>, d: vec3<f32>, t_min: f32, t_max: f32) -> VoxelHit {
    var result: VoxelHit;
    result.hit = false;
    result.steps = 0u;

    let inv = 1.0 / d;
    let brick_level = root.levels;
    let extent = 8 << root.levels;
    let root_t = slab(vec3<f32>(0.0), vec3<f32>(f32(extent)), o, inv);
    let t_start = max(root_t.x, t_min);
    let t_end = min(root_t.y, t_max);
    if t_start > t_end {
        return result;
    }

    let root_address = node_address(root, root.node);
    var mask = topology_load(root_address);
    if (mask & OCTREE_CHILD_MASK) == 0u {
        return result;
    }
    var child_ref = topology_load(root_address + 1u);

    let step = select(vec3<i32>(1), vec3<i32>(-1), d < vec3<f32>(0.0));
    // a cell's far boundary is at cell + 1 when stepping up, cell when stepping down
    let far = (step + vec3<i32>(1)) / 2;

    var stack_mask: array<u32, MAX_OCTREE_DEPTH>;
    var stack_ref: array<u32, MAX_OCTREE_DEPTH>;
    var path = 0u;
    var level = 0u;
    var cells = 2;
    var size = extent / 2;
    var origin = vec3<i32>(0);
    var cell = entry_cell_n(origin, size, 2, o, inv, step, t_start);
    var t_next = next_boundary(origin, cell, size, far, o, inv);

    var steps = 0u;
    loop {
        if steps >= MAX_TRAVERSAL_STEPS {
            break;
        }
        steps += 1u;

        if any(cell < vec3<i32>(0)) || any(cell >= vec3<i32>(cells)) {
            // left this node or brick: resume the parent past the cell we were in
            if level == 0u {
                break;
            }
            level -= 1u;
            cells = 2;
            size = extent >> (level + 1u);
            cell = octant_of((path >> (3u * level)) & 7u);
            origin -= cell * size;
            mask = stack_mask[level];
            child_ref = stack_ref[level];
            t_next = next_boundary(origin, cell, size, far, o, inv);
            cell += step * nearest_axis(t_next);
            t_next = next_boundary(origin, cell, size, far, o, inv);
            continue;
        }

        if level == brick_level {
            let k = u32(cell.x + 8 * cell.y + 64 * cell.z);
            let brick = node_address(root, child_ref);
            let word = topology_load(brick + (k >> 5u));
            if ((word >> (k & 31u)) & 1u) != 0u {
                let lo = origin + cell;
                let tt = slab(vec3<f32>(lo), vec3<f32>(lo + vec3<i32>(1)), o, inv);
                // a voxel; only report it if the ray genuinely touches it within range
                if tt.x <= tt.y && tt.y >= t_start && tt.x <= t_end {
                    var rank = countOneBits(word & ((1u << (k & 31u)) - 1u));
                    for (var w = 0u; w < (k >> 5u); w++) {
                        rank += countOneBits(topology_load(brick + w));
                    }
                    let local = vec3<u32>(lo) & vec3<u32>(3u);
                    result.hit = true;
                    result.t = max(tt.x, t_start);
                    result.cell = vec3<u32>(lo);
                    result.terminal_index = local.x + 4u * local.y + 16u * local.z;
                    result.face = entry_face(lo, lo + vec3<i32>(1), o, inv, step, tt.x, t_start);
                    // offsets stay within the block's page, so this is still a valid handle
                    result.attribute_handle = topology_load(brick + BRICK_MASK_WORDS) + rank;
                    result.steps = steps;
                    return result;
                }
            }
        } else {
            let k = u32(cell.x + 2 * cell.y + 4 * cell.z);
            if ((mask >> k) & 1u) != 0u {
                let lo = origin + cell * size;
                let tt = slab(vec3<f32>(lo), vec3<f32>(lo + vec3<i32>(size)), o, inv);
                let rank = countOneBits(mask & ((1u << k) - 1u));
                stack_mask[level] = mask;
                stack_ref[level] = child_ref;
                path = (path & ~(7u << (3u * level))) | (k << (3u * level));
                level += 1u;
                origin = lo;
                if level == brick_level {
                    cells = 8;
                    size = 1;
                    child_ref = child_ref + rank * BRICK_WORDS;
                } else {
                    size /= 2;
                    let address = node_address(root, child_ref + rank * OCTREE_NODE_WORDS);
                    mask = topology_load(address);
                    child_ref = topology_load(address + 1u);
                }
                cell = entry_cell_n(origin, size, cells, o, inv, step, max(tt.x, t_start));
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

// Bind group 1 and scene-level search. Mirrors `SceneSnapshot::intersect` in
// crates/engine/voxel/src/scene.rs: scene BVH -> (template or refit mini-BVH | flattened frames) -> pose frame ->
// group-local hierarchy (64-tree or octree), keeping the nearest world-distance hit.

import super::types::{
    Bvh4Node, PoseFrame, InstanceRecord, SceneUniform,
    FRAME_SINGLE_PAGE, FRAME_OCTREE, INSTANCE_USES_TEMPLATE, INSTANCE_OWN_BVH, NO_MINI_BVH, MODE_FLATTENED_GROUPS,
};
import super::geometry::template_bvh;
import super::dda::{VoxelRoot, VoxelHit};
import super::tree64::trace_tree64;
import super::octree::trace_octree;
import super::ray::{slab, sanitize_direction, transform_point, transform_vector};

@group(1) @binding(0) var<uniform> scene: SceneUniform;
@group(1) @binding(1) var<storage, read> scene_bvh: array<Bvh4Node>;
@group(1) @binding(2) var<storage, read> frames: array<PoseFrame>;
@group(1) @binding(3) var<storage, read> instances: array<InstanceRecord>;
@group(1) @binding(4) var<storage, read> slot_frames: array<u32>;

const BVH_LEAF_BIT: u32 = 0x80000000u;
const BVH_EMPTY: u32 = 0xFFFFFFFFu;
const INVALID_FRAME: u32 = 0xFFFFFFFFu;
// Median-split BVH4s push at most three siblings per level. 16 entries cover a scene BVH of
// 4^5 leaves; mini-BVHs over a handful of pose groups are only one or two levels deep. Private
// stacks are kept small because they cost occupancy, which this latency-bound traversal needs.
const SCENE_STACK_SIZE: u32 = 16u;
const MINI_STACK_SIZE: u32 = 6u;

struct SceneHit {
    hit: bool,
    t: f32,
    frame: u32,
    voxel: VoxelHit,
    // diagnostics
    scene_nodes: u32,
    group_candidates: u32,
};

// A node's four children ordered nearest first: each slot packs (child reference, t_enter bits),
// with missed children pushed to the end as BVH_EMPTY.
struct OrderedChildren {
    entries: array<vec2<u32>, 4>,
};

// Slab-tests all four children at once (the SoA node layout makes this pure vector math) and
// sorts the hits front to back with a five-comparison sorting network.
fn order_children(node: Bvh4Node, o: vec3<f32>, inv: vec3<f32>, t_min: f32, t_max: f32) -> OrderedChildren {
    let t0x = (node.min_x - o.x) * inv.x;
    let t1x = (node.max_x - o.x) * inv.x;
    let t0y = (node.min_y - o.y) * inv.y;
    let t1y = (node.max_y - o.y) * inv.y;
    let t0z = (node.min_z - o.z) * inv.z;
    let t1z = (node.max_z - o.z) * inv.z;
    let enter = max(max(max(min(t0x, t1x), min(t0y, t1y)), min(t0z, t1z)), vec4<f32>(t_min));
    let exit = min(min(min(max(t0x, t1x), max(t0y, t1y)), max(t0z, t1z)), vec4<f32>(t_max));
    let hit = (enter <= exit) & (node.children != vec4<u32>(BVH_EMPTY));

    var keys = select(vec4<f32>(3.4e38), enter, hit);
    var refs = select(vec4<u32>(BVH_EMPTY), node.children, hit);
    var out: OrderedChildren;
    for (var i = 0u; i < 4u; i++) {
        out.entries[i] = vec2<u32>(refs[i], bitcast<u32>(keys[i]));
    }
    // sorting network on t_enter
    out.entries = sort_pair(out.entries, 0u, 1u);
    out.entries = sort_pair(out.entries, 2u, 3u);
    out.entries = sort_pair(out.entries, 0u, 2u);
    out.entries = sort_pair(out.entries, 1u, 3u);
    out.entries = sort_pair(out.entries, 1u, 2u);
    return out;
}

fn sort_pair(entries: array<vec2<u32>, 4>, a: u32, b: u32) -> array<vec2<u32>, 4> {
    var e = entries;
    if bitcast<f32>(e[b].y) < bitcast<f32>(e[a].y) {
        let swap = e[a];
        e[a] = e[b];
        e[b] = swap;
    }
    return e;
}

// Traces one frame's local hierarchy, whichever kind its root was baked as. The world ray is
// transformed without renormalizing, so the local t is still a world distance.
fn trace_frame(frame_id: u32, o: vec3<f32>, d: vec3<f32>, t_min: f32, t_max: f32) -> VoxelHit {
    let f = frames[frame_id];
    var root: VoxelRoot;
    root.node = f.root_node;
    root.levels = f.levels;
    root.page_base = f.root_page_base;
    root.single_page = (f.flags & FRAME_SINGLE_PAGE) != 0u;
    let local_o = transform_point(f.world_to_local, o);
    let local_d = sanitize_direction(transform_vector(f.world_to_local, d));
    if (f.flags & FRAME_OCTREE) != 0u {
        return trace_octree(root, local_o, local_d, t_min, t_max);
    }
    return trace_tree64(root, local_o, local_d, t_min, t_max);
}

fn try_frame(frame_id: u32, o: vec3<f32>, d: vec3<f32>, t_min: f32, best: ptr<function, SceneHit>) {
    (*best).group_candidates += 1u;
    let hit = trace_frame(frame_id, o, d, t_min, (*best).t);
    if hit.hit && (!(*best).hit || hit.t < (*best).t) {
        (*best).hit = true;
        (*best).t = hit.t;
        (*best).frame = frame_id;
        (*best).voxel = hit;
    }
}

// Per-instance refinement: test the frame's tight current bound before tracing it.
fn try_bounded_frame(frame_id: u32, o: vec3<f32>, d: vec3<f32>, inv: vec3<f32>, t_min: f32, best: ptr<function, SceneHit>) {
    if frame_id == INVALID_FRAME {
        return;
    }
    let f = frames[frame_id];
    let tt = slab(f.bounds_min, f.bounds_max, o, inv);
    if max(tt.x, t_min) > min(tt.y, (*best).t) {
        return;
    }
    try_frame(frame_id, o, d, t_min, best);
}

// Refit mini-BVHs follow the scene BVH in its buffer; templates have their own.
fn mini_bvh_node(own_bvh: bool, index: u32) -> Bvh4Node {
    if own_bvh {
        return scene_bvh[index];
    }
    return template_bvh[index];
}

fn intersect_instance(instance_index: u32, o: vec3<f32>, d: vec3<f32>, inv: vec3<f32>, t_min: f32, best: ptr<function, SceneHit>) {
    let instance = instances[instance_index];

    let own_bvh = (instance.flags & INSTANCE_OWN_BVH) != 0u;
    if (instance.flags & (INSTANCE_USES_TEMPLATE | INSTANCE_OWN_BVH)) == 0u || instance.mini_bvh_root == NO_MINI_BVH {
        // single-root assets and out-of-envelope poses test every slot directly
        for (var slot = 0u; slot < instance.group_count; slot++) {
            try_bounded_frame(slot_frames[instance.slot_base + slot], o, d, inv, t_min, best);
        }
        return;
    }

    // template-shared and per-instance refit mini-BVHs both live in asset root space
    let root_o = transform_point(instance.world_to_root, o);
    let root_inv = 1.0 / sanitize_direction(transform_vector(instance.world_to_root, d));

    var stack: array<vec2<u32>, MINI_STACK_SIZE>;
    var len = 1u;
    stack[0] = vec2<u32>(instance.mini_bvh_root, bitcast<u32>(t_min));
    loop {
        if len == 0u {
            break;
        }
        len -= 1u;
        let entry = stack[len];
        if bitcast<f32>(entry.y) > (*best).t {
            continue;
        }
        let ordered = order_children(mini_bvh_node(own_bvh, entry.x), root_o, root_inv, t_min, (*best).t);
        // visit leaves nearest first so early hits prune the rest
        for (var i = 0u; i < 4u; i++) {
            let child = ordered.entries[i];
            if child.x != BVH_EMPTY && (child.x & BVH_LEAF_BIT) != 0u && bitcast<f32>(child.y) <= (*best).t {
                let slot = child.x & ~BVH_LEAF_BIT;
                try_bounded_frame(slot_frames[instance.slot_base + slot], o, d, inv, t_min, best);
            }
        }
        // push nodes farthest first so the nearest pops first
        for (var i = 4u; i > 0u; i--) {
            let child = ordered.entries[i - 1u];
            if child.x != BVH_EMPTY && (child.x & BVH_LEAF_BIT) == 0u && len < MINI_STACK_SIZE {
                stack[len] = child;
                len += 1u;
            }
        }
    }
}

// Where the point at world position `p` on a frame's geometry was at the previous snapshot. Mirrors
// `ResolvedPoseFrame::previous_position`: group-local geometry is rigid, so the point keeps its
// local position and only the frame moves.
fn previous_world_position(frame_id: u32, p: vec3<f32>) -> vec3<f32> {
    let f = frames[frame_id];
    return transform_point(f.prev_local_to_world, transform_point(f.world_to_local, p));
}

// Finds the nearest hit along a world ray (`query_scene_candidates` + `intersect_instance`).
fn trace_scene(o: vec3<f32>, d_in: vec3<f32>, t_min: f32, t_max: f32) -> SceneHit {
    var best: SceneHit;
    best.hit = false;
    best.t = t_max;
    best.scene_nodes = 0u;
    best.group_candidates = 0u;
    if scene.bvh_node_count == 0u {
        return best;
    }

    let d = sanitize_direction(d_in);
    let inv = 1.0 / d;

    var stack: array<vec2<u32>, SCENE_STACK_SIZE>;
    var len = 1u;
    stack[0] = vec2<u32>(0u, bitcast<u32>(t_min));
    loop {
        if len == 0u {
            break;
        }
        len -= 1u;
        let entry = stack[len];
        if bitcast<f32>(entry.y) > best.t {
            continue;
        }
        best.scene_nodes += 1u;
        let node = scene_bvh[entry.x];

        let ordered = order_children(scene_bvh[entry.x], o, inv, t_min, best.t);
        // visit leaves nearest first so early hits prune the rest
        for (var i = 0u; i < 4u; i++) {
            let child = ordered.entries[i];
            if child.x != BVH_EMPTY && (child.x & BVH_LEAF_BIT) != 0u && bitcast<f32>(child.y) <= best.t {
                let leaf = child.x & ~BVH_LEAF_BIT;
                if scene.mode == MODE_FLATTENED_GROUPS {
                    try_frame(leaf, o, d, t_min, &best);
                } else {
                    intersect_instance(leaf, o, d, inv, t_min, &best);
                }
            }
        }
        // push nodes farthest first so the nearest pops first
        for (var i = 4u; i > 0u; i--) {
            let child = ordered.entries[i - 1u];
            if child.x != BVH_EMPTY && (child.x & BVH_LEAF_BIT) == 0u && len < SCENE_STACK_SIZE {
                stack[len] = child;
                len += 1u;
            }
        }
    }
    return best;
}

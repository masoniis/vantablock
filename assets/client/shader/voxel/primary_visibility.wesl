// PrimaryVisibility: one unchanged world ray per pixel through the scene, writing the resolved
// pose-frame identity, exact integer cell, world distance, and traversal cost. `footprint_lod`
// builds filter each pixel's ray cone through the 64-tree LOD summaries; a filtered hit reports
// its level, the aligned corner of its node as the cell, and its LOD record. In `transmission`
// builds (footprint LOD only) a filtered hit is partly transparent: it covers the pixel by its
// summary's opacity `a`, and the ray continues past its occupied box to a second record, which
// lighting blends in with weight `1 - a`.
//
// Each texture holds the front record in its first channel(s) and the record behind in the next:
// vis.xy / vis.zw = identity, handle (identity VISIBILITY_MISS: nothing, or sky behind)
//   identity = frame (20 bits) << 12 | lod (3 bits) << 9 | terminal cell index (6 bits) << 3 | face (3 bits)
//   handle   = logical handle of the hit voxel's attribute word, or the LOD record index when lod > 0
// depth.x / depth.y = world-ray t_hit (camera ray directions are normalized); depth.z = the front
//   record's coverage `a` (1 unless the hit transmits; the record behind is weighted `1 - a`)
// cell.x / cell.y = the hit voxel's group-local coordinate, x | y << 10 | z << 20
// stats = scene nodes (8 bits) | group candidates (8 bits) << 8 | voxel DDA steps (16 bits) << 16,
//   summed over both traces

import super::lib::types::{ViewUniform, VISIBILITY_MISS, VIEW_STOCHASTIC_FOOTPRINT};
import super::lib::scene::{SceneHit, frames, trace_scene_filtered};
import super::lib::geometry::lod_record_load;
import super::lib::lod::{FootprintFilter, exact_footprint, lod_opacity, lod_filtered_box};
import super::lib::view::camera_ray_direction;
import super::lib::ray::{sanitize_direction, slab, transform_point, transform_vector};

@group(2) @binding(0) var<uniform> view: ViewUniform;
@group(2) @binding(1) var vis_out: texture_storage_2d<rgba32uint, write>;
@group(2) @binding(2) var depth_out: texture_storage_2d<rgba32float, write>;
@group(2) @binding(3) var cell_out: texture_storage_2d<rg32uint, write>;
@group(2) @binding(4) var stats_out: texture_storage_2d<r32uint, write>;

const MISS_DEPTH: f32 = 3.0e38;

// A stable value in [0, 1) per traced pixel (a PCG hash of its coordinates).
fn pixel_hash(pixel: vec2<f32>) -> f32 {
    var h = u32(pixel.x) * 747796405u + u32(pixel.y) * 2891336453u + 2654435769u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    h = (h >> 22u) ^ h;
    return f32(h >> 8u) / 16777216.0;
}

// The ray cone of the traced pixel centered at `pixel`: a pinhole camera's cone has no radius,
// and its diameter at distance t is `footprint_scale` traced pixels' width there. The width comes
// from the traced grid, so reduced render scales filter more. With `VIEW_STOCHASTIC_FOOTPRINT`
// the threshold is the pixel's hash, so a summary of opacity `a` stops a fraction `a` of the rays
// (stochastic transmission) instead of all or none of them.
@ if(footprint_lod)
fn pixel_footprint(pixel: vec2<f32>) -> FootprintFilter {
    let left = camera_ray_direction(view, pixel - vec2<f32>(0.5, 0.0));
    let right = camera_ray_direction(view, pixel + vec2<f32>(0.5, 0.0));
    let threshold = select(view.footprint_threshold, pixel_hash(pixel), (view.flags & VIEW_STOCHASTIC_FOOTPRINT) != 0u);
    return FootprintFilter(0.0, 0.5 * length(right - left) * view.footprint_scale, threshold);
}

// Exact builds trace a zero cone, which `trace_scene_filtered` compiles down to `trace_scene`.
@ if(!footprint_lod)
fn pixel_footprint(pixel: vec2<f32>) -> FootprintFilter {
    return exact_footprint();
}

// What lies behind a footprint-filtered hit that transmits: `(coverage a, world t past its
// occupied box)`, or `a = 1` when the hit is opaque. The ray continues from just past where it
// leaves the node's occupied box (clipped to the frame's bounds, as traversal clips it), so the
// next trace can't report the same node again but finds its neighbors.
@ if(transmission)
fn continuation(hit: SceneHit, o: vec3<f32>, dir: vec3<f32>) -> vec2<f32> {
    if !hit.hit || hit.voxel.lod == 0u {
        return vec2<f32>(1.0, 0.0);
    }
    let frame = frames[hit.frame];
    let record = lod_record_load(hit.voxel.attribute_handle);
    let local_o = transform_point(frame.world_to_local, o);
    let local_d = sanitize_direction(transform_vector(frame.world_to_local, sanitize_direction(dir)));
    let a = lod_opacity(record, local_d);
    if a >= view.transmission_opaque {
        return vec2<f32>(1.0, 0.0);
    }
    let filtered = lod_filtered_box(record, vec3<f32>(hit.voxel.cell), f32(1u << (2u * hit.voxel.lod)));
    let lo = max(filtered[0], vec3<f32>(frame.local_bounds_min));
    let hi = min(filtered[1], vec3<f32>(frame.local_bounds_max));
    let exit = max(slab(lo, hi, local_o, 1.0 / local_d).y, hit.t);
    return vec2<f32>(a, exit + max(exit, 1.0) * 1.0e-5);
}

// The front trace's filter with transmission: every summary the cone covers that has any opacity
// is hit, since what it doesn't cover is blended in from behind.
fn front_footprint(center: vec2<f32>) -> FootprintFilter {
    var cone = pixel_footprint(center);
    cone.threshold = 0.0;
    return cone;
}

// A record as stored: identity, handle, packed cell, and world t (as bits).
fn pack_record(hit: SceneHit) -> vec4<u32> {
    if !hit.hit {
        return vec4<u32>(VISIBILITY_MISS, VISIBILITY_MISS, 0u, bitcast<u32>(MISS_DEPTH));
    }
    let lod = min(hit.voxel.lod, 7u);
    let identity = (hit.frame << 12u) | (lod << 9u) | (hit.voxel.terminal_index << 3u) | hit.voxel.face;
    let cell = hit.voxel.cell.x | (hit.voxel.cell.y << 10u) | (hit.voxel.cell.z << 20u);
    return vec4<u32>(identity, hit.voxel.attribute_handle, cell, bitcast<u32>(hit.t));
}

// A pixel's records (`pack_record`): the front hit, its coverage `a`, the hit behind it (a miss
// unless the front transmits), and the traversal counters of both traces.
struct PixelLayers {
    front: vec4<u32>,
    behind: vec4<u32>,
    a: f32,
    stats: u32,
};

fn add_stats(stats: vec3<u32>, hit: SceneHit) -> vec3<u32> {
    return stats + vec3<u32>(hit.scene_nodes, hit.group_candidates, hit.voxel.steps);
}

fn pack_stats(stats: vec3<u32>) -> u32 {
    return min(stats.x, 255u) | (min(stats.y, 255u) << 8u) | (min(stats.z, 65535u) << 16u);
}

// Up to two traces through one call site, so the scene traversal is instantiated once, keeping
// only the packed front record live across the second.
@ if(transmission)
fn trace_pixel(dir: vec3<f32>, center: vec2<f32>) -> PixelLayers {
    var none: SceneHit;
    none.hit = false;
    var layers = PixelLayers(vec4<u32>(0u), pack_record(none), 1.0, 0u);
    var stats = vec3<u32>(0u);
    var cone = front_footprint(center);
    var t_min = 0.0;
    for (var layer = 0u; layer < 2u; layer++) {
        let hit = trace_scene_filtered(view.camera_position, dir, t_min, MISS_DEPTH, cone);
        stats = add_stats(stats, hit);
        if layer == 1u {
            layers.behind = pack_record(hit);
            break;
        }
        layers.front = pack_record(hit);
        let through = continuation(hit, view.camera_position, dir);
        if through.x >= 1.0 {
            break;
        }
        layers.a = through.x;
        t_min = through.y;
        cone = pixel_footprint(center);
    }
    layers.stats = pack_stats(stats);
    return layers;
}

@ if(!transmission)
fn trace_pixel(dir: vec3<f32>, center: vec2<f32>) -> PixelLayers {
    var none: SceneHit;
    none.hit = false;
    let hit = trace_scene_filtered(view.camera_position, dir, 0.0, MISS_DEPTH, pixel_footprint(center));
    return PixelLayers(pack_record(hit), pack_record(none), 1.0, pack_stats(add_stats(vec3<u32>(0u), hit)));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(vis_out);
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let pixel = vec2<i32>(id.xy);
    // the temporal resolve jitters every ray within its pixel (zero without it)
    let center = vec2<f32>(id.xy) + 0.5 + view.jitter;
    let layers = trace_pixel(camera_ray_direction(view, center), center);
    let front = layers.front;
    let behind = layers.behind;
    textureStore(stats_out, pixel, vec4<u32>(layers.stats, 0u, 0u, 0u));
    textureStore(vis_out, pixel, vec4<u32>(front.xy, behind.xy));
    textureStore(depth_out, pixel, vec4<f32>(bitcast<f32>(front.w), bitcast<f32>(behind.w), layers.a, 0.0));
    textureStore(cell_out, pixel, vec4<u32>(front.z, behind.z, 0u, 0u));
}

// PrimaryVisibility: one unchanged world ray per pixel through the scene, writing the resolved
// pose-frame identity, exact integer cell, world distance, and traversal cost.
//
// vis.x = frame (20 bits) << 12 | lod (3 bits) << 9 | terminal cell index (6 bits) << 3 | face (3 bits)
// vis.y = logical handle of the hit voxel's attribute word
// depth = world-ray t_hit (camera ray directions are normalized)
// cell  = the hit voxel's group-local coordinate, x | y << 10 | z << 20
// stats = scene nodes (8 bits) | group candidates (8 bits) << 8 | voxel DDA steps (16 bits) << 16

import super::lib::types::{ViewUniform, VISIBILITY_MISS};
import super::lib::scene::trace_scene;
import super::lib::view::camera_ray_direction;

@group(2) @binding(0) var<uniform> view: ViewUniform;
@group(2) @binding(1) var vis_out: texture_storage_2d<rg32uint, write>;
@group(2) @binding(2) var depth_out: texture_storage_2d<r32float, write>;
@group(2) @binding(3) var cell_out: texture_storage_2d<r32uint, write>;
@group(2) @binding(4) var stats_out: texture_storage_2d<r32uint, write>;

const MISS_DEPTH: f32 = 3.0e38;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(vis_out);
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let pixel = vec2<i32>(id.xy);
    let dir = camera_ray_direction(view, vec2<f32>(id.xy) + 0.5);
    let hit = trace_scene(view.camera_position, dir, 0.0, MISS_DEPTH);

    let stats = min(hit.scene_nodes, 255u) | (min(hit.group_candidates, 255u) << 8u)
        | (min(hit.voxel.steps, 65535u) << 16u);
    textureStore(stats_out, pixel, vec4<u32>(stats, 0u, 0u, 0u));

    if !hit.hit {
        textureStore(vis_out, pixel, vec4<u32>(VISIBILITY_MISS, VISIBILITY_MISS, 0u, 0u));
        textureStore(depth_out, pixel, vec4<f32>(MISS_DEPTH, 0.0, 0.0, 0.0));
        textureStore(cell_out, pixel, vec4<u32>(0u));
        return;
    }

    let lod = 0u;
    let identity = (hit.frame << 12u) | (lod << 9u) | (hit.voxel.terminal_index << 3u) | hit.voxel.face;
    let cell = hit.voxel.cell.x | (hit.voxel.cell.y << 10u) | (hit.voxel.cell.z << 20u);
    textureStore(vis_out, pixel, vec4<u32>(identity, hit.voxel.attribute_handle, 0u, 0u));
    textureStore(depth_out, pixel, vec4<f32>(hit.t, 0.0, 0.0, 0.0));
    textureStore(cell_out, pixel, vec4<u32>(cell, 0u, 0u, 0u));
}

// DirectLighting: shades the visibility buffer with sun/sky light, shadow rays through the same
// scene, emission, and simple aerial perspective. Also renders the debug views.

import super::lib::types::{ViewUniform, Environment, VISIBILITY_MISS, VIEW_TONEMAP_IN_SHADER};
import super::lib::geometry::{materials, voxel_material, lod_record_load};
import super::lib::lod::{lod_material, lod_normal};
import super::lib::scene::{frames, trace_scene, previous_world_position};
import super::lib::view::{camera_ray_direction, sky_color, tonemap, id_color, heatmap};

@group(2) @binding(0) var<uniform> view: ViewUniform;
@group(2) @binding(1) var<uniform> env: Environment;
@group(2) @binding(2) var vis_tex: texture_2d<u32>;
@group(2) @binding(3) var depth_tex: texture_2d<f32>;
@group(2) @binding(4) var cell_tex: texture_2d<u32>;
@group(2) @binding(5) var stats_tex: texture_2d<u32>;

// must match DebugView in crates/game/client/src/render/settings.rs
const DEBUG_SHADED: u32 = 0u;
const DEBUG_NORMALS: u32 = 1u;
const DEBUG_POSE_FRAMES: u32 = 2u;
const DEBUG_VOXEL_CELLS: u32 = 3u;
const DEBUG_TRAVERSAL_STEPS: u32 = 4u;
const DEBUG_GROUP_CANDIDATES: u32 = 5u;
const DEBUG_SCENE_NODES: u32 = 6u;
const DEBUG_DEPTH: u32 = 7u;
const DEBUG_MOTION: u32 = 8u;
const DEBUG_LOD_LEVELS: u32 = 9u;

const FOG_DENSITY: f32 = 0.0035;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    // one triangle covering the screen
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOutput;
    out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

fn face_normal(face: u32) -> vec3<f32> {
    let axis = face / 2u;
    let sign = select(-1.0, 1.0, (face & 1u) == 1u);
    var n = vec3<f32>(0.0);
    n[axis] = sign;
    return n;
}

fn shade(pixel: vec2<i32>, dir: vec3<f32>) -> vec3<f32> {
    let vis = textureLoad(vis_tex, pixel, 0).xy;
    let stats = textureLoad(stats_tex, pixel, 0).x;

    if view.debug_view == DEBUG_TRAVERSAL_STEPS {
        return heatmap(f32(stats >> 16u) / 256.0);
    }
    if view.debug_view == DEBUG_GROUP_CANDIDATES {
        return heatmap(f32((stats >> 8u) & 0xFFu) / 24.0);
    }
    if view.debug_view == DEBUG_SCENE_NODES {
        return heatmap(f32(stats & 0xFFu) / 32.0);
    }
    if vis.x == VISIBILITY_MISS {
        return sky_color(dir, env);
    }

    let frame_id = vis.x >> 12u;
    let lod = (vis.x >> 9u) & 7u;
    let face = vis.x & 7u;
    let frame = frames[frame_id];
    let t = textureLoad(depth_tex, pixel, 0).x;

    // local normals reach world space through the inverse-transpose: the rows of world_to_local
    // as columns
    let to_world = mat3x3<f32>(frame.world_to_local[0].xyz, frame.world_to_local[1].xyz, frame.world_to_local[2].xyz);
    // the hit face's normal, which offsets shadow rays off the surface
    var face_n = -dir;
    if face < 6u {
        face_n = normalize(to_world * face_normal(face));
    }
    // a footprint-filtered hit (lod > 0) shades with its LOD summary's material and mean normal
    // (the hit face's where the summary's faces cancel out); exact hits with the voxel's
    var normal = face_n;
    var material_index: u32;
    if lod == 0u {
        material_index = voxel_material(vis.y);
    } else {
        let record = lod_record_load(vis.y);
        material_index = lod_material(record);
        let n = lod_normal(record);
        if any(n != vec3<f32>(0.0)) {
            normal = normalize(to_world * n);
        }
    }

    switch view.debug_view {
        case DEBUG_NORMALS: {
            return normal * 0.5 + 0.5;
        }
        case DEBUG_POSE_FRAMES: {
            return id_color(frame_id);
        }
        case DEBUG_VOXEL_CELLS: {
            let c = textureLoad(cell_tex, pixel, 0).x;
            let cell = vec3<u32>(c & 0x3FFu, (c >> 10u) & 0x3FFu, c >> 20u);
            return id_color((cell.x * 73856093u) ^ (cell.y * 19349663u) ^ (cell.z * 83492791u)) * 0.8 + 0.2;
        }
        case DEBUG_DEPTH: {
            return vec3<f32>(exp(-t * 0.05));
        }
        case DEBUG_LOD_LEVELS: {
            // exact voxels grey, filtered hits from blue (level 1) to red (level 4 and up)
            if lod == 0u {
                return vec3<f32>(0.25);
            }
            return heatmap(f32(lod - 1u) / 3.0);
        }
        case DEBUG_MOTION: {
            // where this surface point was last frame, through the hit frame's pose history and
            // the camera's: red/green encode the screen-space motion in pixels, blue its length
            let position = view.camera_position + dir * t;
            let previous = previous_world_position(frame_id, position);
            let now_clip = view.clip_from_world * vec4<f32>(position, 1.0);
            let then_clip = view.prev_clip_from_world * vec4<f32>(previous, 1.0);
            let motion = (now_clip.xy / now_clip.w - then_clip.xy / then_clip.w) * 0.5 * view.viewport;
            return vec3<f32>(clamp(motion / 16.0, vec2<f32>(-1.0), vec2<f32>(1.0)) * 0.5 + 0.5, clamp(length(motion) / 32.0, 0.0, 1.0));
        }
        default: {}
    }

    let material = materials[frame.palette_base + material_index];
    let position = view.camera_position + dir * t;
    // half a voxel off the surface, in world units
    let voxel_size = 1.0 / length(frame.world_to_local[0].xyz);
    let light = normalize(env.light_direction);

    let n_dot_l = max(dot(normal, light), 0.0);
    var sun = 0.0;
    if n_dot_l > 0.0 {
        let shadow = trace_scene(position + face_n * voxel_size * 0.5, light, 0.0, 1.0e30);
        sun = select(n_dot_l, 0.0, shadow.hit);
    }
    // sky light from the hemisphere the surface faces, plus a little ground bounce
    let sky = mix(env.horizon, env.zenith, normal.y * 0.5 + 0.5) * env.ambient;
    let bounce = env.horizon * env.ambient * 0.15 * max(-normal.y, 0.0);

    var color = material.rgb * (env.light_color * sun + sky + bounce) + material.rgb * material.a;
    let fog = 1.0 - exp(-t * FOG_DENSITY);
    color = mix(color, sky_color(dir, env), fog);
    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // the visibility buffer may be traced below target resolution; shade with the exact ray
    // that produced the sampled visibility texel so depth and identity stay consistent
    let pixel = vec2<i32>(floor(in.position.xy * view.target_to_grid));
    var color = shade(pixel, camera_ray_direction(view, vec2<f32>(pixel) + 0.5));
    if (view.flags & VIEW_TONEMAP_IN_SHADER) != 0u && view.debug_view == DEBUG_SHADED {
        color = tonemap(color);
    }
    return vec4<f32>(color, 1.0);
}

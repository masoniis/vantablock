// Camera rays and output helpers shared by the voxel passes.

import super::types::{ViewUniform, Environment};

// The world-space direction through a point of the traced ray grid (use pixel centers). The ray
// starts at the camera, and its parameter t is a world distance.
fn camera_ray_direction(view: ViewUniform, pixel: vec2<f32>) -> vec3<f32> {
    let uv = pixel / view.viewport;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    // bevy uses infinite reverse-z, so the near plane sits at ndc z = 1
    let near = view.world_from_clip * vec4<f32>(ndc, 1.0, 1.0);
    return normalize(near.xyz / near.w - view.camera_position);
}

fn sky_color(dir: vec3<f32>, env: Environment) -> vec3<f32> {
    let gradient = pow((dir.y + 1.0) * 0.5, 0.9);
    var color = mix(env.horizon, env.zenith, gradient);

    let sun = max(dot(dir, normalize(env.sun_direction)), 0.0);
    color += pow(sun, 1024.0) * env.sun_disk + pow(sun, 64.0) * env.sun_disk * 0.16;

    let moon = max(dot(dir, normalize(env.moon_direction)), 0.0);
    color += pow(moon, 800.0) * vec3<f32>(0.9, 0.9, 0.95) + pow(moon, 32.0) * vec3<f32>(0.1, 0.2, 0.4) * 0.15;
    return color;
}

// Narkowicz's ACES fit, for targets bevy will not tonemap.
fn tonemap(color: vec3<f32>) -> vec3<f32> {
    let x = color * 0.8;
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

// A stable, distinct color per integer ID for debug views.
fn id_color(id: u32) -> vec3<f32> {
    var h = id * 0x9E3779B9u;
    h ^= h >> 16u;
    h *= 0x7FEB352Du;
    h ^= h >> 15u;
    return vec3<f32>(f32(h & 0xFFu), f32((h >> 8u) & 0xFFu), f32((h >> 16u) & 0xFFu)) / 255.0;
}

// Blue (cheap) to red (expensive).
fn heatmap(x: f32) -> vec3<f32> {
    let t = clamp(x, 0.0, 1.0);
    return clamp(vec3<f32>(4.0 * t - 2.0, 2.0 - abs(4.0 * t - 2.0), 2.0 - 4.0 * t), vec3<f32>(0.0), vec3<f32>(1.0));
}

// TemporalResolve: temporal anti-aliasing over the traced ray grid. Primary visibility jitters
// every ray within its pixel by a sequence of subpixel offsets (`ViewUniform::jitter`), lighting
// shades the jittered rays and reports where each pixel's surface was last frame
// (direct_lighting.wesl `history_offset`), and this pass blends the reprojected history with the
// current frame. Over a still view the history converges to the pixel's box-filtered average,
// what a supersampled frame shows.
//
// History that no longer matches what the pixel sees (disocclusion, shading changes) is clipped
// toward the box spanned by the current frame's 3x3 neighborhood (mean +- CLIP_GAMMA standard
// deviations in YCoCg). The history's alpha counts the frames it averages, so a pixel converges
// as a running mean until the current frame's weight reaches `ViewUniform::temporal_blend`.

import super::lib::types::{ViewUniform, VIEW_HISTORY_VALID};

@group(0) @binding(0) var<uniform> view: ViewUniform;
@group(0) @binding(1) var current_tex: texture_2d<f32>;
@group(0) @binding(2) var offset_tex: texture_2d<f32>;
@group(0) @binding(3) var history_tex: texture_2d<f32>;
@group(0) @binding(4) var history_sampler: sampler;
@group(0) @binding(5) var resolved_out: texture_storage_2d<rgba16float, write>;

const CLIP_GAMMA: f32 = 1.5;
// the most frames the history's alpha counts
const MAX_SAMPLES: f32 = 64.0;

fn to_ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.25 * c.r + 0.5 * c.g + 0.25 * c.b, 0.5 * c.r - 0.5 * c.b, -0.25 * c.r + 0.5 * c.g - 0.25 * c.b);
}

fn from_ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

// Moves `h` toward the center of the box `[lo, hi]` until it lies inside it.
fn clip_to_box(h: vec3<f32>, lo: vec3<f32>, hi: vec3<f32>) -> vec3<f32> {
    let center = 0.5 * (lo + hi);
    let extent = 0.5 * (hi - lo) + vec3<f32>(1.0e-5);
    let v = h - center;
    let units = abs(v / extent);
    let m = max(units.x, max(units.y, units.z));
    if m > 1.0 {
        return center + v / m;
    }
    return h;
}

// The history at `position` (in traced pixels) through a Catmull-Rom filter, from five bilinear
// taps (the 4x4 footprint without its corners, renormalized). Bilinear resampling every frame
// would blur a moving camera's history more each frame.
fn sample_history(position: vec2<f32>, grid: vec2<f32>) -> vec4<f32> {
    let p1 = floor(position - 0.5) + 0.5;
    let f = position - p1;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let p0 = (p1 - 1.0) / grid;
    let p3 = (p1 + 2.0) / grid;
    let p12 = (p1 + w2 / w12) / grid;
    var sum = textureSampleLevel(history_tex, history_sampler, vec2<f32>(p12.x, p0.y), 0.0) * (w12.x * w0.y);
    sum += textureSampleLevel(history_tex, history_sampler, vec2<f32>(p0.x, p12.y), 0.0) * (w0.x * w12.y);
    sum += textureSampleLevel(history_tex, history_sampler, p12, 0.0) * (w12.x * w12.y);
    sum += textureSampleLevel(history_tex, history_sampler, vec2<f32>(p3.x, p12.y), 0.0) * (w3.x * w12.y);
    sum += textureSampleLevel(history_tex, history_sampler, vec2<f32>(p12.x, p3.y), 0.0) * (w12.x * w3.y);
    let weight = w12.x * w0.y + w0.x * w12.y + w12.x * w12.y + w3.x * w12.y + w12.x * w3.y;
    return max(sum / weight, vec4<f32>(0.0));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(resolved_out);
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let pixel = vec2<i32>(id.xy);
    let current = textureLoad(current_tex, pixel, 0).rgb;

    // the current frame's neighborhood moments
    let last = vec2<i32>(size) - 1;
    var m1 = vec3<f32>(0.0);
    var m2 = vec3<f32>(0.0);
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let c = to_ycocg(textureLoad(current_tex, clamp(pixel + vec2<i32>(dx, dy), vec2<i32>(0), last), 0).rgb);
            m1 += c;
            m2 += c * c;
        }
    }
    let mean = m1 / 9.0;
    let sigma = sqrt(max(m2 / 9.0 - mean * mean, vec3<f32>(0.0)));

    let then = vec2<f32>(pixel) + 0.5 + textureLoad(offset_tex, pixel, 0).xy;
    let grid = vec2<f32>(size);
    var resolved = current;
    var samples = 0.0;
    // debug views show the current frame alone
    if (view.flags & VIEW_HISTORY_VALID) != 0u && view.debug_view == 0u && all(then >= vec2<f32>(0.0)) && all(then <= grid) {
        let history = sample_history(then, grid);
        let clipped = clip_to_box(to_ycocg(history.rgb), mean - CLIP_GAMMA * sigma, mean + CLIP_GAMMA * sigma);
        samples = history.a;
        let weight = max(1.0 / (samples + 1.0), view.temporal_blend);
        resolved = mix(from_ycocg(clipped), current, weight);
    }
    textureStore(resolved_out, pixel, vec4<f32>(resolved, min(samples + 1.0, MAX_SAMPLES)));
}

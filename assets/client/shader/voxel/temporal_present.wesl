// TemporalPresent: shows the temporal resolve's image (temporal_resolve.wesl) in the view target,
// each target pixel showing the traced pixel it falls in.

import super::lib::types::ViewUniform;

@group(0) @binding(0) var<uniform> view: ViewUniform;
@group(0) @binding(1) var resolved_tex: texture_2d<f32>;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy * view.target_to_grid));
    return vec4<f32>(textureLoad(resolved_tex, pixel, 0).rgb, 1.0);
}

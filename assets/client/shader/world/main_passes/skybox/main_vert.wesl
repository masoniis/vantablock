struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,

    // pass the clip-space position (without perspective divide)
    // to the fragment shader so it can un-project it.
    @location(0) clip_pos_for_unproject: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32
) -> VertexOutput {
    var out: VertexOutput;

    // full screen quad
    let vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), // triangle 1
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), // triangle 2
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );

    let pos = vertices[in_vertex_index];

    // the full screen quad is at the maximum distance (0.0 for inverse z)
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.clip_pos_for_unproject = out.clip_position;

    return out;
}

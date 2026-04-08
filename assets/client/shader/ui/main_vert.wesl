import package::ui::common::{ortho_projection, Object, objects, material, Material};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // scale model matrix (screen space) by the orthographic projection to get clip space
    let model = objects[instance_index].model;
    out.clip_position = ortho_projection * model * vec4<f32>(position, 0.0, 1.0);

    return out;
}

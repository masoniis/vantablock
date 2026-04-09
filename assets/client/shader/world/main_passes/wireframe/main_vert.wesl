import super::bindings::{camera, objects, VertexInput};
import package::world::lib::wireframe_vertex::{WireframeVertex};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    vertex: WireframeVertex,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // basic transform to world space from uniform buffer
    let model_mat = objects[instance_index].model_matrix;

    // INFO: ----------------
    //         output
    // ----------------------

    var out: VertexOutput;

    // small depth bias so the lines are rendered on top of filled geometry
    let depth_bias = 0.0005;
    out.clip_position = camera.view_proj * model_mat * vec4(vertex.position, 1.0);
    out.clip_position.z += depth_bias * out.clip_position.w;

    out.color = vertex.color;

    return out;
}

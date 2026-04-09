/// A uniform buffer containing camera data that is consistent across a single view.
struct CameraUniform {
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
    render_distance: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

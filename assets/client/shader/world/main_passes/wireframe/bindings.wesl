// per-view data (camera)
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// per-object data (model transforms)
struct ObjectData {
    model_matrix: mat4x4<f32>,
};
@group(2) @binding(0)
var<storage, read> objects: array<ObjectData>;

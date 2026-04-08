// per-view data
@group(0) @binding(0)
var<uniform> ortho_projection: mat4x4<f32>;

// per-material data
struct Material {
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: Material;

// per-object data
struct Object {
    model: mat4x4<f32>,
};
@group(2) @binding(0)
var<storage, read> objects: array<Object>;

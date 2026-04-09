/// A uniform buffer containing sun camera data that is consistent across a single view.
struct SunCameraUniform {
    view_proj: mat4x4<f32>,
};

import super::sun_camera_uniform::{SunCameraUniform};

/// Calculates a shadow value based on a fragment world pos and the sun camera.
///
/// Returns 1.0 if area is lit, 0.0 if it is shadowed.
fn calculate_shadow_from_world(
    world_pos: vec3<f32>,
    shadow_cam: SunCameraUniform,
    shadow_tex: texture_depth_2d,
    shadow_samp: sampler_comparison
) -> f32 {
    // transform world position to shadow map space
    let shadow_pos = shadow_cam.view_proj * vec4<f32>(world_pos, 1.0);

    // perspective divide and transform to texture coordinates [0, 1]
    let shadow_coords = shadow_pos.xyz / shadow_pos.w;
    let uv = shadow_coords.xy * 0.5 + 0.5;
    let uv_flipped = vec2<f32>(uv.x, 1.0 - uv.y); // flip Y for texture coordinates

    // check if fragment is outside the shadow frustum in ANY dimension.
    var shadow_factor = 1.0;
    if uv.x > 1.0 || uv.x < 0.0 || uv.y > 1.0 || uv.y < 0.0 || shadow_coords.z > 1.0 {
        // outside the frustum, so leave shadow_factor at 1.0 (lit)
    } else {
        // inside the frustum, perform depth comparison to see if frag shadow Z is LEQ shadow texture Z
        shadow_factor = textureSampleCompare(
            shadow_tex,
            shadow_samp,
            uv_flipped,
            shadow_coords.z
        );
    }

    return shadow_factor;
}

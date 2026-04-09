use crate::player::CAMERA_NEAR_PLANE;
use crate::prelude::*;
use crate::render::data::ExtractedSun;
use crate::render::pipeline::main_passes::opaque_pass::queue::Opaque3dRenderPhase;
use crate::render::pipeline::shadow_pass::gpu_resources::SHADOW_MAP_RESOLUTION;
use crate::render::pipeline::shadow_pass::gpu_resources::{ShadowViewBuffer, ShadowViewData};
use bevy::ecs::prelude::*;
use bevy::math::Vec4Swizzles;
use bevy::render::renderer::RenderQueue;
use bevy::render::view::ExtractedView;

/// The max distance at which shadows render
const SHADOW_DISTANCE: f32 = 256.0;

#[instrument(skip_all)]
pub fn update_shadow_view_buffer_system(
    // input
    view_buffer: Res<ShadowViewBuffer>,
    view_query: Query<(&ExtractedView, &Opaque3dRenderPhase)>,
    sun: Res<ExtractedSun>,

    // output (writing buffer to queue)
    queue: Res<RenderQueue>,
) {
    let Some((extracted_view, _)) = view_query.iter().next() else {
        return;
    };

    // INFO: -------------------------
    //         sun view matrix
    // -------------------------------
    // NOTE: goal is to create a view matrix of the sun "looking at the world"

    let sun_direction = Vec3::from_array(sun.main_light_direction).normalize_or_zero();

    // stable up direction
    let light_up = if sun_direction.y.abs() > 0.999 {
        Vec3::Z
    } else {
        Vec3::Y
    };

    // view mat
    let camera_position = extracted_view.world_from_view.translation();
    let sun_target = camera_position;
    let sun_position = sun_target + sun_direction * 1024.0; // sun is "far away" from target
    let sun_view_mat = Mat4::look_at_rh(sun_position, sun_target, light_up);

    // camera view and inverse matrices
    let view_matrix = extracted_view.world_from_view.to_matrix().inverse();
    let projection_matrix = extracted_view.clip_from_view;
    let view_proj = projection_matrix * view_matrix;
    let inv_view_proj = view_proj.inverse();

    // INFO: ------------------------------
    //         frustum bounding box
    // ------------------------------------
    // NOTE: goal is to create a bounding box for the shadow texture that fits
    // the camera frustum in order to be efficient with the texture map

    // far plane is close to 0, but not 0 since 0 is "infinite" away
    // using the infinite reverse z projection
    let z_ndc_far = CAMERA_NEAR_PLANE / SHADOW_DISTANCE;
    let frustum_corners_ndc: [Vec4; 8] = [
        // near plane (z=1.0)
        vec4(-1.0, -1.0, 1.0, 1.0),
        vec4(1.0, -1.0, 1.0, 1.0),
        vec4(-1.0, 1.0, 1.0, 1.0),
        vec4(1.0, 1.0, 1.0, 1.0),
        // far plane (z=z_ndc_far)
        vec4(-1.0, -1.0, z_ndc_far, 1.0),
        vec4(1.0, -1.0, z_ndc_far, 1.0),
        vec4(-1.0, 1.0, z_ndc_far, 1.0),
        vec4(1.0, 1.0, z_ndc_far, 1.0),
    ];

    // find the bounding box in sun-view space
    let mut min_light_space = Vec3::splat(f32::MAX);
    let mut max_light_space = Vec3::splat(f32::MIN);

    for &corner_ndc in frustum_corners_ndc.iter() {
        // ndc -> world space
        let world_pos_w: Vec4 = inv_view_proj * corner_ndc;
        let world_pos: Vec3 = world_pos_w.xyz() / world_pos_w.w;

        // world space -> sun view space
        let light_space_pos_w: Vec4 = sun_view_mat * world_pos.extend(1.0);
        let light_space_pos: Vec3 = light_space_pos_w.xyz();

        // find the min/max of the box in sun space
        min_light_space = min_light_space.min(light_space_pos);
        max_light_space = max_light_space.max(light_space_pos);
    }

    // INFO: ------------------------------
    //         texel snapping logic
    // ------------------------------------
    // NOTE: goal is to snap the ortho projection to the shadow map's texel
    // grid which reducing movement when the camera pans and shifts

    // size of the ortho box (pre-snap)
    let box_size_x = max_light_space.x - min_light_space.x;
    let box_size_y = max_light_space.y - min_light_space.y;

    // size of a single "texel" in light/sun space
    let texel_size_x = box_size_x / SHADOW_MAP_RESOLUTION as f32;
    let texel_size_y = box_size_y / SHADOW_MAP_RESOLUTION as f32;

    // snap both min and max to the texel grid
    let snapped_min_x = (min_light_space.x / texel_size_x).floor() * texel_size_x;
    let snapped_min_y = (min_light_space.y / texel_size_y).floor() * texel_size_y;
    let snapped_max_x = snapped_min_x + box_size_x;
    let snapped_max_y = snapped_min_y + box_size_y;

    // INFO: ---------------------------------
    //         shadow ortho projection
    // ---------------------------------------

    // looking down the sun'z -Z axis (as if we are sun)
    // max.z is the nearest point to the sun
    // min.z is the furthest

    // light space values are in the -Z, so they are negative
    let near_plane = -max_light_space.z;
    let far_plane = -min_light_space.z;

    let light_proj_matrix = Mat4::orthographic_rh(
        snapped_min_x,
        snapped_max_x, // left, right
        snapped_min_y,
        snapped_max_y, // bottom, top
        near_plane,
        far_plane,
    );

    // INFO: ---------------------
    //         upload data
    // ---------------------------

    let light_view_proj_matrix = light_proj_matrix * sun_view_mat;
    let shadow_data = ShadowViewData {
        light_view_proj_matrix: light_view_proj_matrix.to_cols_array(),
    };

    queue.write_buffer(&view_buffer.buffer, 0, bytemuck::cast_slice(&[shadow_data]));
}

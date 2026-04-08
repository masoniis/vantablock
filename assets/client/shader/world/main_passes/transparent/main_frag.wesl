import package::world::main_passes::lib::geometry_bindings::{
  environment, texture_array, texture_sampler, camera
};
import package::world::main_passes::lib::environment_uniform::{
  calculate_sky_color
};

@fragment
fn fs_main(
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) @interpolate(flat) texture_index: u32,
    @location(4) world_pos: vec3<f32>,
    @location(5) clip_pos_for_fog: vec4<f32>,
) -> @location(0) vec4<f32> {
    // INFO: --------------------------
    //         texture sampling
    // --------------------------------

    let texture_color: vec4<f32> = textureSample(texture_array, texture_sampler, tex_coords, texture_index);
    if texture_color.a < 0.01 {
      discard;
    }

    // INFO: -------------
    //         fog
    // -------------------

    let ray_dir = normalize((camera.inverse_view_proj * clip_pos_for_fog).xyz);
    let fog_color = calculate_sky_color(ray_dir, environment);

    let distance_from_camera = distance(world_pos, camera.world_position);
    let fog_amount = smoothstep(camera.render_distance * 0.75, camera.render_distance, distance_from_camera);
    let final_color = mix(texture_color, vec4(fog_color, 1.0), fog_amount);

    return final_color;
}

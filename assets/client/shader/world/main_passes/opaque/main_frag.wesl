import package::world::main_passes::lib::geometry_bindings::{
  texture_array, texture_sampler, camera, environment, shadow_camera, shadow_map, shadow_sampler
};
import package::world::main_passes::lib::environment_uniform::{
  calculate_sky_color
};
import package::world::shadow::lib::calculate_shadow::{calculate_shadow_from_world};

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

    let texture_color: vec4<f32> = textureSample(
        texture_array, texture_sampler, tex_coords, texture_index
    );
    let base_color = texture_color.rgb * color;

    // INFO: ------------------------------
    //         directional lighting
    // ------------------------------------

    let L = normalize(environment.main_light_direction);
    let N = normalize(normal);

    let ambient_strength = environment.ambient_strength;
    let diff_amount = max(dot(N, L), 0.0);
    let diffuse_color = environment.main_light_color * diff_amount;

    var lit_color: vec3<f32>;

    let shadow_factor: f32 = calculate_shadow_from_world(
        world_pos, shadow_camera, shadow_map, shadow_sampler
    );
    if shadow_factor == 1.0 {
        // not in shadow, apply full lighting
        let light_contribution = vec3<f32>(ambient_strength) + diffuse_color;
        lit_color = base_color * light_contribution;
    } else {
        // in shadow, apply only ambient light
        lit_color = base_color * ambient_strength;
    }

    // INFO: -------------
    //         fog
    // -------------------

    let ray_dir = normalize((camera.inverse_view_proj * clip_pos_for_fog).xyz);
    let fog_color = calculate_sky_color(ray_dir, environment);

    let distance_from_camera = distance(world_pos, camera.world_position);
    let fog_amount: f32 = smoothstep(
        camera.render_distance * 0.75, camera.render_distance, distance_from_camera
    );
    let final_color_rgb = mix(lit_color, fog_color, fog_amount);

    return vec4<f32>(final_color_rgb, texture_color.a);
}

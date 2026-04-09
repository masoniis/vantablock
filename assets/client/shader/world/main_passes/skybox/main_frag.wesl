import super::super::lib::environment_uniform::{calculate_sky_color, EnvironmentUniform};
import super::super::lib::camera_uniform::{camera};

@group(1) @binding(0)
var<uniform> environment: EnvironmentUniform;
@fragment
fn fs_main(
    @location(0) clip_position: vec4<f32>,
) -> @location(0) vec4<f32> {
    // direction vector from camera to fragment in world space
    let ray_dir = normalize((camera.inverse_view_proj * clip_position).xyz);

    // render sun
    let sun_dir_norm = normalize(environment.sun_direction);
    let sun_dot = dot(ray_dir, sun_dir_norm);

    let sun_color = environment.sun_disk_color;
    let sun_halo_color = environment.sun_disk_color * 0.8;

    let sun_disk = pow(max(0.0, sun_dot), 1024.0) * sun_color;
    let sun_halo = pow(max(0.0, sun_dot), 64.0) * sun_halo_color * 0.2;

    // render moon
    let moon_dir_norm = normalize(environment.moon_direction);
    let moon_dot = dot(ray_dir, moon_dir_norm);

    let moon_base_color = vec3<f32>(0.9, 0.9, 0.95);
    let moon_disk = pow(max(0.0, moon_dot), 800.0) * moon_base_color;

    let moon_glow_color = vec3<f32>(0.1, 0.2, 0.4);
    let moon_halo = pow(max(0.0, moon_dot), 32.0) * moon_glow_color * 0.15;

    // compose everything with sky color
    let sky_color = calculate_sky_color(ray_dir, environment);
    let final_color = sky_color + sun_disk + sun_halo + moon_disk + moon_halo;

    return vec4<f32>(final_color, 1.0);
}

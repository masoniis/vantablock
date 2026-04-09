struct EnvironmentUniform {
    main_light_direction: vec3<f32>,
    time: f32,
    main_light_color: vec3<f32>,
    ambient_strength: f32,
    sun_direction: vec3<f32>,
    sun_disk_color: vec3<f32>,
    moon_direction: vec3<f32>,
    horizon_color: vec3<f32>,
    zenith_color: vec3<f32>,
};

fn calculate_sky_color(
    /// normalized direction from camera to a point in the sky
    dir: vec3<f32>,
		/// environment parameters
    env: EnvironmentUniform
) -> vec3<f32> {
		// convert to [0.0, 1.0] range
    let vertical_gradient = (dir.y + 1.0) * 0.5;

		// apply gradient curve
    let eased_gradient = pow(vertical_gradient, 0.90);

		// interpolate
    let sky_color = mix(env.horizon_color, env.zenith_color, eased_gradient);
    return sky_color;
}

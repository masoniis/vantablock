use crate::prelude::*;
use bevy::ecs::prelude::Resource;
use bevy::render::extract_resource::ExtractResource;
use shared::time::WorldClockResource;

/// Controls where the sun/sky starts LERPing to the horizon
const SUN_HEIGHT_DAY: f32 = 0.2;
/// Controls where the LERP from regular sun/sky to horizon sun/sky ends
/// and the start of the LERP to twighlight (full night) begins
const SUN_HEIGHT_HORIZON: f32 = -0.05;
/// Controls where the LERP from horizon to twighlight ends
const SUN_HEIGHT_TWILIGHT: f32 = -0.15;

/// The night-time ambient intensity
const NIGHT_AMBIENT_INTENSITY: f32 = 0.2;
/// The horizon-fade ambient intensity
const HORIZON_AMBIENT_INTENSITY: f32 = 0.5;

struct SkyPalette {
    day_zenith: Vec3,
    day_horizon: Vec3,
    sunset_zenith: Vec3,
    sunset_horizon: Vec3,
    night_zenith: Vec3,
    night_horizon: Vec3,
}

const SKY: SkyPalette = SkyPalette {
    day_zenith: Vec3::new(0.2, 0.5, 0.9),
    day_horizon: Vec3::new(0.7, 0.8, 0.95),
    sunset_zenith: Vec3::new(0.3, 0.2, 0.3),
    sunset_horizon: Vec3::new(1.0, 0.5, 0.2),
    night_zenith: Vec3::new(0.0, 0.0, 0.02),
    night_horizon: Vec3::new(0.02, 0.02, 0.05),
};

struct SunMoonPalette {
    day_color: Vec3,
    sunset_color: Vec3,
    moon_color: Vec3,
}

const SUN: SunMoonPalette = SunMoonPalette {
    day_color: Vec3::new(1.0, 0.95, 0.9),
    sunset_color: Vec3::new(1.0, 0.6, 0.3),
    moon_color: Vec3::new(0.1, 0.2, 0.4),
};

#[derive(Resource, Debug, Default, PartialEq, Clone)]
pub struct ExtractedSun {
    pub main_light_direction: [f32; 3],
    pub main_light_color: [f32; 3],
    pub ambient_strength: f32,
    pub sun_direction: [f32; 3],
    pub sun_disk_color: [f32; 3],
    pub moon_direction: [f32; 3],
    pub zenith: [f32; 3],
    pub horizon: [f32; 3],
}

impl ExtractResource for ExtractedSun {
    type Source = WorldClockResource;

    fn extract_resource(source: &Self::Source) -> Self {
        let cycle_progress = source.day_night_cycle_value();
        // angle is offset by pi/2 to make 0 midnight rather than sunrise
        let angle = (cycle_progress * 2.0 * PI) - (FRAC_PI_2);
        let sun_height = angle.sin();
        let sun_horizontal = angle.cos();

        let sun_vec = Vec3::new(sun_horizontal, sun_height, 0.0);
        let moon_vec = Vec3::new(-sun_horizontal, -sun_height, 0.0);

        // INFO: color calculations
        let (final_zenith, final_horizon) = calculate_sky_gradient(sun_height);
        let (main_dir, main_color, main_strength, sun_disk) =
            calculate_lighting(sun_height, sun_vec, moon_vec);

        ExtractedSun {
            sun_direction: sun_vec.normalize_or_zero().to_array(),
            moon_direction: moon_vec.normalize_or_zero().to_array(),
            main_light_direction: main_dir.normalize_or_zero().to_array(),
            main_light_color: main_color.to_array(),
            ambient_strength: main_strength,
            sun_disk_color: sun_disk.to_array(),
            zenith: final_zenith.to_array(),
            horizon: final_horizon.to_array(),
        }
    }
}

/// Computes the sky gradient colors by interpolating the sky palettes based on the three sky phases
fn calculate_sky_gradient(h: f32) -> (Vec3, Vec3) {
    if h > SUN_HEIGHT_HORIZON {
        // first gradient phase
        //
        // lerp between day and sunset palette
        let t = inverse_lerp(SUN_HEIGHT_DAY, SUN_HEIGHT_HORIZON, h);
        (
            SKY.day_zenith.lerp(SKY.sunset_zenith, t),
            SKY.day_horizon.lerp(SKY.sunset_horizon, t),
        )
    } else if h > SUN_HEIGHT_TWILIGHT {
        // second gradient phase
        //
        // lerp between sunset and night palette
        let t = inverse_lerp(SUN_HEIGHT_HORIZON, SUN_HEIGHT_TWILIGHT, h);
        (
            SKY.sunset_zenith.lerp(SKY.night_zenith, t),
            // non-linear horizon power to fade out faster
            SKY.sunset_horizon.lerp(SKY.night_horizon, t.powf(0.75)),
        )
    } else {
        // just night time palette
        (SKY.night_zenith, SKY.night_horizon)
    }
}

fn calculate_lighting(h: f32, sun_vec: Vec3, moon_vec: Vec3) -> (Vec3, Vec3, f32, Vec3) {
    if h > SUN_HEIGHT_HORIZON {
        // first sun phase (towards horizon)
        //
        // lerp between sun_color and sunset color
        // lerp between max ambient and horizon ambient intensity
        let t = inverse_lerp(SUN_HEIGHT_DAY, SUN_HEIGHT_HORIZON, h);
        let light_col = SUN.day_color.lerp(SUN.sunset_color, t);
        let intensity = 1.0_f32.lerp(HORIZON_AMBIENT_INTENSITY, t);

        (sun_vec, light_col, intensity, light_col)
    } else if h > SUN_HEIGHT_TWILIGHT {
        // second sun phase (sunset to twilight)
        //
        // lerp between horizon color and twighlight color
        // lerp between sunset ambient and night ambient intensity
        let t = inverse_lerp(SUN_HEIGHT_HORIZON, SUN_HEIGHT_TWILIGHT, h);
        let light_col = SUN.sunset_color.lerp(Vec3::new(0.3, 0.1, 0.05), t);
        let intensity = 0.5_f32.lerp(NIGHT_AMBIENT_INTENSITY, t);

        (sun_vec, light_col, intensity, light_col * intensity)
    } else {
        // moon phase, (full twilight)
        //
        // lerp between
        let night_depth = ((h - SUN_HEIGHT_TWILIGHT) / -0.2).clamp(0.0, 1.0);
        let intensity = 0.2_f32.lerp(NIGHT_AMBIENT_INTENSITY, night_depth);

        (moon_vec, SUN.moon_color, intensity, Vec3::ZERO)
    }
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    ((value - a) / (b - a)).clamp(0.0, 1.0)
}

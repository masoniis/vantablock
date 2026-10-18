//! Sun, moon, and sky colors derived from the world clock.

use crate::render::gpu::layout::GpuEnvironment;
use bevy::{math::FloatExt, prelude::*, render::extract_resource::ExtractResource};
use common::time::WorldClockResource;
use std::f32::consts::{FRAC_PI_2, PI};

/// Where the sun/sky start fading toward the horizon palette.
const SUN_HEIGHT_DAY: f32 = 0.2;
/// End of the day-to-horizon fade, start of the fade to twilight.
const SUN_HEIGHT_HORIZON: f32 = -0.05;
/// End of the horizon-to-twilight fade.
const SUN_HEIGHT_TWILIGHT: f32 = -0.15;
const NIGHT_AMBIENT: f32 = 0.2;
const HORIZON_AMBIENT: f32 = 0.5;
/// The sun's orbit is tilted so noon light rakes across surfaces instead of hitting straight down.
const ORBIT_TILT: f32 = 0.45;

const DAY_ZENITH: Vec3 = Vec3::new(0.2, 0.5, 0.9);
const DAY_HORIZON: Vec3 = Vec3::new(0.7, 0.8, 0.95);
const SUNSET_ZENITH: Vec3 = Vec3::new(0.3, 0.2, 0.3);
const SUNSET_HORIZON: Vec3 = Vec3::new(1.0, 0.5, 0.2);
const NIGHT_ZENITH: Vec3 = Vec3::new(0.0, 0.0, 0.02);
const NIGHT_HORIZON: Vec3 = Vec3::new(0.02, 0.02, 0.05);
const SUN_DAY: Vec3 = Vec3::new(1.0, 0.95, 0.88);
const SUN_SUNSET: Vec3 = Vec3::new(1.0, 0.6, 0.3);
const MOON: Vec3 = Vec3::new(0.1, 0.2, 0.4);

/// The environment as the GPU sees it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ExtractedEnvironment(pub GpuEnvironment);

impl ExtractResource for ExtractedEnvironment {
    type Source = WorldClockResource;

    fn extract_resource(clock: &Self::Source) -> Self {
        Self(environment_at(
            clock.day_night_cycle_value(),
            clock.animation_seconds(),
        ))
    }
}

/// The environment at `cycle` (0 = midnight, 0.5 = noon).
pub fn environment_at(cycle: f32, time: f32) -> GpuEnvironment {
    // offset so 0 is midnight rather than sunrise
    let angle = cycle * 2.0 * PI - FRAC_PI_2;
    let sun = Vec3::new(
        angle.cos(),
        angle.sin() * ORBIT_TILT.cos(),
        angle.sin() * ORBIT_TILT.sin(),
    )
    .normalize();
    let moon = -sun;
    let h = sun.y;

    let (zenith, horizon) = if h > SUN_HEIGHT_HORIZON {
        let t = inverse_lerp(SUN_HEIGHT_DAY, SUN_HEIGHT_HORIZON, h);
        (
            DAY_ZENITH.lerp(SUNSET_ZENITH, t),
            DAY_HORIZON.lerp(SUNSET_HORIZON, t),
        )
    } else if h > SUN_HEIGHT_TWILIGHT {
        let t = inverse_lerp(SUN_HEIGHT_HORIZON, SUN_HEIGHT_TWILIGHT, h);
        (
            SUNSET_ZENITH.lerp(NIGHT_ZENITH, t),
            SUNSET_HORIZON.lerp(NIGHT_HORIZON, t.powf(0.75)),
        )
    } else {
        (NIGHT_ZENITH, NIGHT_HORIZON)
    };

    let (light_direction, light_color, ambient, sun_disk) = if h > SUN_HEIGHT_HORIZON {
        let t = inverse_lerp(SUN_HEIGHT_DAY, SUN_HEIGHT_HORIZON, h);
        let color = SUN_DAY.lerp(SUN_SUNSET, t);
        (sun, color * 2.2, 1.0.lerp(HORIZON_AMBIENT, t), color)
    } else if h > SUN_HEIGHT_TWILIGHT {
        let t = inverse_lerp(SUN_HEIGHT_HORIZON, SUN_HEIGHT_TWILIGHT, h);
        let color = SUN_SUNSET.lerp(Vec3::new(0.3, 0.1, 0.05), t);
        let ambient = HORIZON_AMBIENT.lerp(NIGHT_AMBIENT, t);
        (sun, color * 2.2, ambient, color * ambient)
    } else {
        (moon, MOON, NIGHT_AMBIENT, Vec3::ZERO)
    };

    GpuEnvironment {
        light_direction: light_direction.to_array(),
        ambient,
        light_color: light_color.to_array(),
        time,
        zenith: zenith.to_array(),
        horizon: horizon.to_array(),
        sun_direction: sun.to_array(),
        sun_disk: sun_disk.to_array(),
        moon_direction: moon.to_array(),
        ..default()
    }
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    ((value - a) / (b - a)).clamp(0.0, 1.0)
}

use crate::simulation_world::{
    input::{ActionStateResource, SimulationAction},
    player::{ActiveCamera, CameraComponent},
    terrain::{ActiveTerrainGenerator, TerrainGeneratorLibrary},
    time::{world_clock::SECONDS_IN_A_DAY, WorldClockResource},
};
use bevy_ecs::prelude::*;
use glam::Vec3;
use std::time::Duration;

struct Showcase {
    generator_idx: usize,
    time_of_day: f32,
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

const SHOWCASES: &[Showcase] = &[
    Showcase {
        // basic flat area for introduction
        generator_idx: 0,
        time_of_day: 0.25, // showcase sun shining from left
        position: Vec3::new(0.0, 68.5, 0.0),
        yaw: 85.0,
        pitch: -12.0,
    },
    Showcase {
        // sinwave with vertex waving and shadows
        generator_idx: 1,
        time_of_day: 0.3, // showcase sun shining from left
        position: Vec3::new(-661.0, 68.5, 175.0),
        yaw: 85.0,
        pitch: -12.0,
    },
    Showcase {
        // realistic terrain gen with horizon
        generator_idx: 2,  // realistic terrain
        time_of_day: 0.73, // sunset
        position: Vec3::new(939.0, 71.2, 1218.0),
        yaw: -169.0,
        pitch: -3.0,
    },
    Showcase {
        // showcase 3d simplex noise to feature 3d terrain
        generator_idx: 4, // 3d simplex
        time_of_day: 0.25,
        position: Vec3::new(1425.0, 73.0, 1984.0),
        yaw: 48.0,
        pitch: 7.0,
    },
    Showcase {
        // cool bump thing
        generator_idx: 3,
        time_of_day: 0.223,
        position: Vec3::new(3693.0, 140.0, 507.0),
        yaw: 0.0,
        pitch: -3.0,
    },
    Showcase {
        // cool mountain hole thing
        generator_idx: 3,
        time_of_day: 0.25,
        position: Vec3::new(37252.0, 138.0, -2488.0),
        yaw: -147.0,
        pitch: -3.0,
    },
];

pub fn apply_showcase_system(
    action_state: Res<ActionStateResource>,
    mut active_cam_q: Query<&mut CameraComponent>,
    active_camera: Res<ActiveCamera>,
    mut active_generator: ResMut<ActiveTerrainGenerator>,
    terrain_gen_lib: Res<TerrainGeneratorLibrary>,
    mut world_clock: ResMut<WorldClockResource>,
) {
    let showcase_idx = if action_state.just_happened(SimulationAction::Showcase1) {
        1
    } else if action_state.just_happened(SimulationAction::Showcase2) {
        2
    } else if action_state.just_happened(SimulationAction::Showcase3) {
        3
    } else if action_state.just_happened(SimulationAction::Showcase4) {
        4
    } else if action_state.just_happened(SimulationAction::Showcase5) {
        5
    } else if action_state.just_happened(SimulationAction::Showcase0) {
        0
    } else {
        return;
    };

    let showcase = &SHOWCASES[showcase_idx];

    // set shaper
    if let Some(generator) = terrain_gen_lib.generators.get(showcase.generator_idx) {
        active_generator.0 = generator.clone();
    }

    // set time of day
    world_clock.time_of_day = Duration::from_secs_f32(SECONDS_IN_A_DAY * showcase.time_of_day);

    // set camera position and rotation
    if let Ok(mut cam) = active_cam_q.get_mut(active_camera.0) {
        cam.position = showcase.position;
        cam.yaw = showcase.yaw;
        cam.pitch = showcase.pitch;
    }
}

pub fn apply_default_showcase_system(
    mut active_cam_q: Query<&mut CameraComponent>,
    active_camera: Res<ActiveCamera>,
    mut active_generator: ResMut<ActiveTerrainGenerator>,
    terrain_gen_lib: Res<TerrainGeneratorLibrary>,
    mut world_clock: ResMut<WorldClockResource>,
) {
    let showcase = &SHOWCASES[1];

    // set shaper
    if let Some(generator) = terrain_gen_lib.generators.get(showcase.generator_idx) {
        active_generator.0 = generator.clone();
    }

    // set time of day
    world_clock.time_of_day = Duration::from_secs_f32(SECONDS_IN_A_DAY * showcase.time_of_day);

    // set camera position and rotation
    if let Ok(mut cam) = active_cam_q.get_mut(active_camera.0) {
        cam.position = showcase.position;
        cam.yaw = showcase.yaw;
        cam.pitch = showcase.pitch;
    }
}

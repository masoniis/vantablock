// geometry bindings is bindings for the main passes (opaque/transparent geometry)

import package::world::shadow::lib::sun_camera_uniform::{SunCameraUniform};
import super::environment_uniform::{EnvironmentUniform};
import super::camera_uniform::{CameraUniform};

// INFO: -----------------------
//         per-view data
// -----------------------------

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> shadow_camera: SunCameraUniform;

@group(0) @binding(2)
var shadow_map: texture_depth_2d;

@group(0) @binding(3)
var shadow_sampler: sampler_comparison;

// INFO: ------------------------------
//         per-environment data
// ------------------------------------

@group(1) @binding(0)
var<uniform> environment: EnvironmentUniform;

// INFO: ---------------------------
//         per-material data
// ---------------------------------

@group(2) @binding(0)
var texture_array: texture_2d_array<f32>;

@group(2) @binding(1)
var texture_sampler: sampler;

// INFO: -------------------------
//         per-object data
// -------------------------------

struct Chunk {
    world_pos: vec3<f32>,
    start_index: u32,
};

@group(3) @binding(0)
var<storage, read> chunks: array<Chunk>;

@group(3) @binding(1)
var<storage, read> faces: array<u32>;
